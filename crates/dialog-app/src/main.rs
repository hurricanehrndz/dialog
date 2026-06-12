#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use dialog_core::{compat, config::Config, parser};
use std::collections::HashSet;

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

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running dialog");
}
