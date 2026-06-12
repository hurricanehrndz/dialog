#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use dialog_core::{
    commandfile::{default_path, CommandFileTail},
    commands::{self, Command},
    compat,
    config::Config,
    exit_codes,
    output::UserInput,
    parser,
    state::DialogState,
};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

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
        // dialog-core layout & text
        "small",
        "big",
        "mini",
        "style",
        "titlefont",
        "messagefont",
        "quitkey",
        // command-file IPC & progress
        "commandfile",
        "progress",
        "progresstext",
        "infotext",
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

/// Open a markdown link in the default browser (web schemes only — the
/// webview must never navigate or launch arbitrary local programs).
#[tauri::command]
fn open_link(url: String) {
    if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
        open_url(&url);
    } else {
        eprintln!("WARNING: refusing to open non-web link {url}");
    }
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

/// Window-level commands the content applier can't handle (quit, resize,
/// reposition, activate). Runs on the watcher thread via AppHandle.
fn handle_shell_command(handle: &tauri::AppHandle, cmd: &Command) {
    let Some(window) = handle.get_webview_window("main") else {
        return;
    };
    match cmd {
        Command::Quit => {
            let app = handle.state::<App>();
            quit(&app, exit_codes::QUIT_COMMAND);
        }
        Command::Activate => {
            let _ = window.set_focus();
        }
        Command::Width(w) => {
            if let Ok(size) = window.inner_size() {
                let scale = window.scale_factor().unwrap_or(1.0);
                let h = size.height as f64 / scale;
                let _ = window.set_size(tauri::LogicalSize::new(*w, h));
            }
        }
        Command::Height(h) => {
            if let Ok(size) = window.inner_size() {
                let scale = window.scale_factor().unwrap_or(1.0);
                let w = size.width as f64 / scale;
                let _ = window.set_size(tauri::LogicalSize::new(w, *h));
            }
        }
        Command::Position(anchor) => position_window(&window, anchor, 16.0),
        _ => {}
    }
}

/// Place the window at one of swiftDialog's nine screen anchors.
fn position_window(window: &tauri::WebviewWindow, anchor: &str, offset: f64) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let Ok(win) = window.outer_size() else {
        return;
    };
    let screen = monitor.size();
    let origin = monitor.position();
    let scale = monitor.scale_factor();
    let off = offset * scale;
    let (sw, sh) = (screen.width as f64, screen.height as f64);
    let (ww, wh) = (win.width as f64, win.height as f64);

    let x = match anchor {
        "topleft" | "left" | "bottomleft" => off,
        "top" | "center" | "centre" | "bottom" => (sw - ww) / 2.0,
        "topright" | "right" | "bottomright" => sw - ww - off,
        _ => return,
    };
    let y = match anchor {
        "topleft" | "top" | "topright" => off,
        "left" | "center" | "centre" | "right" => (sh - wh) / 2.0,
        "bottomleft" | "bottom" | "bottomright" => sh - wh - off,
        _ => return,
    };
    let _ = window.set_position(tauri::PhysicalPosition::new(
        origin.x + x as i32,
        origin.y + y as i32,
    ));
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

    let command_file_path = config
        .value("commandfile")
        .map(|v| std::path::PathBuf::from(v.into_owned()))
        .unwrap_or_else(default_path);

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
        .invoke_handler(tauri::generate_handler![get_state, ui_event, open_link])
        .setup(move |tauri_app| {
            let w = &state.window;
            // swiftDialog windows are chromeless (no title bar); dragging
            // is offered only with --moveable via a webview drag region.
            let mut builder = WebviewWindowBuilder::new(tauri_app, "main", WebviewUrl::default())
                .title("dialog")
                .inner_size(w.width, w.height)
                .resizable(w.resizable)
                .always_on_top(w.ontop)
                .decorations(false)
                .center();
            if let Some(appearance) = &w.appearance {
                builder = builder.theme(match appearance.as_str() {
                    "dark" => Some(tauri::Theme::Dark),
                    "light" => Some(tauri::Theme::Light),
                    _ => None,
                });
            }
            let window = builder.build()?;
            if let Some(anchor) = &w.position {
                position_window(&window, anchor, w.position_offset);
            }

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

            // Command-file watcher: tail for verbs, mutate state, push the
            // full state to the renderer. 250 ms poll keeps well within the
            // spec's 1 s budget (notify-based wakeups can come later).
            let command_path = command_file_path.clone();
            let handle = tauri_app.handle().clone();
            std::thread::spawn(move || {
                let mut tail = match CommandFileTail::open(&command_path) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(
                            "WARNING: cannot watch command file {}: {e}",
                            command_path.display()
                        );
                        return;
                    }
                };
                loop {
                    for line in tail.poll() {
                        let Some(cmd) = commands::parse_line(&line) else {
                            eprintln!("DEBUG: ignoring command line: {line}");
                            continue;
                        };
                        let app = handle.state::<App>();
                        let content_handled = {
                            let mut dialog = app.dialog.lock().unwrap();
                            commands::apply(&mut dialog, &cmd)
                        };
                        if content_handled {
                            let snapshot = app.dialog.lock().unwrap().clone();
                            let _ = handle.emit("state", snapshot);
                        } else {
                            handle_shell_command(&handle, &cmd);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running dialog");
}
