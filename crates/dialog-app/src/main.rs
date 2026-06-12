#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use dialog_core::{
    compat, config::Config, exit_codes, output::UserInput, parser, state::DialogState,
};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Options actually wired up end-to-end. Anything parsed but absent here
/// warns and no-ops (degrade-don't-break). Grows as capabilities land.
fn implemented() -> HashSet<&'static str> {
    [
        "version",
        "help",
        "verbose",
        "json",
        "jsonfile",
        "jsonstring",
        // dialog-core content rendered by the shell
        "title",
        "subtitle",
        "message",
        "messagealignment",
        "messageposition",
        // buttons-actions
        "button1text",
        "button1action",
        "button1disabled",
        "button2",
        "button2text",
        "button2action",
        "button2disabled",
        "infobutton",
        "infobuttontext",
        "infobuttonaction",
        "quitoninfo",
        "timer",
        "hidetimerbar",
        // window-behavior basics
        "width",
        "height",
        "ontop",
        "moveable",
        "resizable",
        "appearance",
        "hideicon",
    ]
    .into_iter()
    .collect()
}

/// On Windows the binary is GUI-subsystem (no console flash), so stdout is
/// detached by default; attach to the invoking console so the JSON output
/// contract works from PowerShell/cmd (cli-compatibility spec).
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };
    unsafe {
        // Only attach when stdout is not already wired up: AttachConsole
        // resets the std handles, which would clobber a pipe the invoking
        // shell set up to capture our output ($x = & dialog.exe ...).
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout.is_null() || stdout == INVALID_HANDLE_VALUE {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

fn help_text() -> String {
    let mut out = String::from(
        "dialog — swiftDialog-compatible dialog utility\n\nUsage: dialog [options]\n\nOptions:\n",
    );
    for def in dialog_core::options::OPTIONS {
        if def.hidden || def.alias_of.is_some() {
            continue;
        }
        let short = def.short.map(|s| format!("-{s}, ")).unwrap_or_default();
        let value = match def.kind {
            dialog_core::options::Kind::Value => " <value>",
            dialog_core::options::Kind::Flag => "",
        };
        out.push_str(&format!("  {short}--{}{value}\n", def.long));
    }
    out
}

/// Shared app state for command handlers.
struct App {
    dialog: Mutex<DialogState>,
    /// Collected user input, printed on exit per the output contract.
    input: Mutex<UserInput>,
    json_output: bool,
    quit_on_info: bool,
}

/// Print collected output (if any) and exit with the given contract code.
fn quit(app: &App, code: i32) -> ! {
    let input = app.input.lock().unwrap();
    if !input.is_empty() {
        println!("{}", input.render(app.json_output));
    }
    std::process::exit(code);
}

fn open_url(url: &str) {
    if let Err(e) = open::that_detached(url) {
        eprintln!("WARNING: failed to open {url}: {e}");
    }
}

#[tauri::command]
fn get_state(app: tauri::State<App>) -> DialogState {
    app.dialog.lock().unwrap().clone()
}

#[tauri::command]
fn ui_event(app: tauri::State<App>, event: String) {
    match event.as_str() {
        "button1" => {
            let action = app.dialog.lock().unwrap().button1.action.clone();
            if let Some(url) = action.filter(|u| !u.is_empty()) {
                open_url(&url);
            }
            quit(&app, exit_codes::BUTTON1);
        }
        "button2" => {
            let action = app.dialog.lock().unwrap().button2.action.clone();
            if let Some(url) = action.filter(|u| !u.is_empty()) {
                open_url(&url);
            }
            quit(&app, exit_codes::BUTTON2);
        }
        "info" => {
            let action = app.dialog.lock().unwrap().info_button.action.clone();
            if let Some(url) = action.filter(|u| !u.is_empty()) {
                open_url(&url);
            }
            if app.quit_on_info {
                quit(&app, exit_codes::INFO_BUTTON);
            }
        }
        "quitkey" => quit(&app, exit_codes::QUIT_KEY),
        other => eprintln!("WARNING: unknown ui event {other}"),
    }
}

fn main() {
    #[cfg(windows)]
    attach_parent_console();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parser::parse(&argv);

    if args.present("version") {
        println!("{VERSION}");
        std::process::exit(0);
    }
    if args.present("help") {
        print!("{}", help_text());
        std::process::exit(0);
    }

    if args.present("verbose") {
        for token in &args.ignored {
            eprintln!("WARNING: ignoring unknown argument {token}");
        }
    }

    let config = match Config::load(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.message());
            std::process::exit(e.exit_code());
        }
    };

    for warning in
        compat::degrade_warnings(&config.cli, &implemented(), compat::Platform::current())
    {
        eprintln!("{warning}");
    }

    let state = DialogState::from_config(&config);
    let app = App {
        json_output: config.present("json"),
        quit_on_info: config.present("quitoninfo"),
        dialog: Mutex::new(state.clone()),
        input: Mutex::new(UserInput::default()),
    };

    // SIGTERM/console-termination → exit 40 (AppVariables exit40).
    ctrlc::set_handler(|| std::process::exit(exit_codes::SIGTERM))
        .expect("failed to install termination handler");

    tauri::Builder::default()
        .manage(app)
        .invoke_handler(tauri::generate_handler![get_state, ui_event])
        .setup(move |tauri_app| {
            let w = &state.window;
            let mut builder = WebviewWindowBuilder::new(tauri_app, "main", WebviewUrl::default())
                .title("dialog")
                .inner_size(w.width, w.height)
                .resizable(w.resizable)
                .always_on_top(w.ontop)
                .center();
            if let Some(appearance) = &w.appearance {
                builder = builder.theme(match appearance.as_str() {
                    "dark" => Some(tauri::Theme::Dark),
                    "light" => Some(tauri::Theme::Light),
                    _ => None,
                });
            }
            builder.build()?;

            // Rust-authoritative timer: the frontend bar is cosmetic; the
            // exit (code 4) fires here regardless of webview health.
            if let Some(timer) = &state.timer {
                let secs = timer.seconds;
                let handle = tauri_app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
                    let app = handle.state::<App>();
                    quit(&app, exit_codes::TIMER);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running dialog");
}
