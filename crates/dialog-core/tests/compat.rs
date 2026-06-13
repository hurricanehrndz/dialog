//! Compatibility verification harness (design D8): the table-driven contract
//! pin. Each case asserts a surface that scripts actually depend on —
//!   - argv → `DialogState` (parse + JSON overlay + defaults),
//!   - command-file scripts → state transitions,
//!   - collected input → the exit stdout the binary prints.
//!
//! Fixtures are drawn from real community deployment scripts (Dan Snelson's
//! Setup-Your-Mac, Trevor Sysock's Baseline) and swiftDialog's own docs, so a
//! regression on a real-world invocation fails *here*, headless, without a
//! display. Reference contract: swiftDialog v3.0.1. The on-display rendering
//! and live swiftDialog side-by-side comparison run in CI (see ci.yml).

use dialog_core::commands::{apply, parse_line};
use dialog_core::compat::{degrade_warnings, DegradeReason, Platform};
use dialog_core::config::Config;
use dialog_core::output::UserInput;
use dialog_core::parser::parse;
use dialog_core::state::DialogState;
use std::collections::HashSet;

fn state(args: &[&str]) -> DialogState {
    DialogState::from_config(&Config::load(parse(args.iter().copied())).unwrap())
}

/// Apply a command-file script (one verb per line) onto an initial dialog,
/// ignoring malformed lines exactly as the watcher does.
fn run_script(args: &[&str], script: &[&str]) -> DialogState {
    let mut s = state(args);
    for line in script {
        if let Some(cmd) = parse_line(line) {
            apply(&mut s, &cmd);
        }
    }
    s
}

/// The stdout a button1 exit prints, after `edits` mutate the live input
/// values (simulating what the user typed/picked before clicking OK).
fn button1_stdout(args: &[&str], edits: impl Fn(&mut DialogState), as_json: bool) -> String {
    let mut s = state(args);
    edits(&mut s);
    UserInput::from_dialog(&s, true).render(as_json)
}

// ── argv → DialogState ──────────────────────────────────────────────────

struct StateCase {
    name: &'static str,
    args: &'static [&'static str],
    check: fn(&DialogState),
}

const STATE_CASES: &[StateCase] = &[
    StateCase {
        name: "docs: minimal title/message/button",
        args: &[
            "--title",
            "Hello",
            "--message",
            "World",
            "--button1text",
            "Done",
        ],
        check: |s| {
            assert_eq!(s.title.as_deref(), Some("Hello"));
            assert_eq!(s.message, "World");
            assert_eq!(s.button1.text, "Done");
            assert!(s.button1.visible && !s.button2.visible);
        },
    },
    StateCase {
        name: "docs: two-button alert with SF icon",
        args: &[
            "--title",
            "Restart?",
            "--message",
            "Save your work.",
            "--icon",
            "SF=exclamationmark.triangle",
            "--button1text",
            "Restart",
            "--button2text",
            "Later",
        ],
        check: |s| {
            assert!(s.button2.visible);
            assert_eq!(s.button2.text, "Later");
            assert_eq!(s.icon.source, "SF=exclamationmark.triangle");
        },
    },
    StateCase {
        name: "docs: mini style forces 540x128",
        args: &["--mini", "--title", "Quick", "--message", "Compact"],
        check: |s| {
            assert!(s.mini);
            assert_eq!((s.window.width, s.window.height), (540.0, 128.0));
        },
    },
    StateCase {
        name: "Setup-Your-Mac: banner + progress + checklist",
        args: &[
            "--bannerimage",
            "/Library/branding/banner.png",
            "--bannertitle",
            "Setting up your Mac",
            "--message",
            "Please wait while we configure your device.",
            "--icon",
            "SF=gear",
            "--button1text",
            "Please Wait",
            "--button1disabled",
            "--progress",
            "6",
            "--progresstext",
            "Initializing…",
            "--listitem",
            "Configuration",
            "--listitem",
            "Applications",
            "--listitem",
            "Security Settings",
            "--moveable",
            "--ontop",
            "--quitkey",
            "k",
        ],
        check: |s| {
            let banner = s.banner.as_ref().expect("banner present");
            assert_eq!(banner.title.as_deref(), Some("Setting up your Mac"));
            assert_eq!(banner.source, "/Library/branding/banner.png");
            assert!(!s.button1.enabled, "button1 disabled during setup");
            let p = s.progress.as_ref().expect("progress present");
            assert_eq!(p.total, 6.0);
            assert_eq!(p.text, "Initializing…");
            assert_eq!(p.current, None, "starts indeterminate");
            assert_eq!(s.list_items.len(), 3);
            assert_eq!(s.quit_key, "k");
            assert!(s.window.ontop && s.window.moveable);
        },
    },
    StateCase {
        name: "Baseline: list with sub-option statuses",
        args: &[
            "--title",
            "Baseline",
            "--message",
            "Running install tasks…",
            "--progress",
            "2",
            "--listitem",
            "Rosetta 2,status=wait,statustext=Installing",
            "--listitem",
            "Company Portal,status=pending",
        ],
        check: |s| {
            assert_eq!(s.list_items.len(), 2);
            assert_eq!(s.list_items[0].title, "Rosetta 2");
            assert_eq!(s.list_items[0].status, "wait");
            assert_eq!(s.list_items[0].status_text, "Installing");
            assert_eq!(s.list_items[1].status, "pending");
        },
    },
    StateCase {
        name: "user-input: textfield + select + checkbox form",
        args: &[
            "--title",
            "Enrollment",
            "--textfield",
            "Computer Name,required,prompt=e.g. MBP-1234",
            "--selecttitle",
            "Department",
            "--selectvalues",
            "HR,Engineering,Operations",
            "--selectdefault",
            "Engineering",
            "--checkbox",
            "Enable FileVault,checked",
        ],
        check: |s| {
            assert_eq!(s.text_fields.len(), 1);
            assert!(s.text_fields[0].required);
            assert_eq!(s.text_fields[0].prompt.as_deref(), Some("e.g. MBP-1234"));
            assert_eq!(s.selects[0].values.len(), 3);
            assert_eq!(s.selects[0].selected, "Engineering");
            assert!(s.checkboxes[0].checked);
        },
    },
    StateCase {
        name: "json: --jsonstring drives the whole dialog",
        args: &[
            "--jsonstring",
            r#"{"title":"From JSON","message":"Body","button1text":"Go","ontop":true,"width":500}"#,
        ],
        check: |s| {
            assert_eq!(s.title.as_deref(), Some("From JSON"));
            assert_eq!(s.button1.text, "Go");
            assert!(s.window.ontop);
            assert_eq!(s.window.width, 500.0);
        },
    },
];

#[test]
fn argv_to_state_contract() {
    for c in STATE_CASES {
        println!("state case: {}", c.name);
        (c.check)(&state(c.args));
    }
}

// ── command-file scripts → state transitions ────────────────────────────

struct ScriptCase {
    name: &'static str,
    args: &'static [&'static str],
    script: &'static [&'static str],
    check: fn(&DialogState),
}

const SCRIPT_CASES: &[ScriptCase] = &[
    ScriptCase {
        name: "Setup-Your-Mac: drive checklist + progress to completion",
        args: &[
            "--progress",
            "3",
            "--listitem",
            "Configuration",
            "--listitem",
            "Applications",
            "--listitem",
            "Security Settings",
        ],
        script: &[
            "listitem: index: 0, status: wait, statustext: Working",
            "progress: 1",
            "listitem: index: 0, status: success, statustext: Done",
            "listitem: index: 1, status: wait",
            "progress: increment",
            "listitem: title: Security Settings, status: error, statustext: Failed",
            "progresstext: Finishing up",
            "this is not a command", // malformed → ignored
        ],
        check: |s| {
            assert_eq!(s.list_items[0].status, "success");
            assert_eq!(s.list_items[0].status_text, "Done");
            assert_eq!(s.list_items[1].status, "wait");
            assert_eq!(s.list_items[2].status, "error");
            assert_eq!(s.list_items[2].status_text, "Failed");
            assert_eq!(s.progress.as_ref().unwrap().current, Some(2.0));
            assert_eq!(s.progress.as_ref().unwrap().text, "Finishing up");
        },
    },
    ScriptCase {
        name: "docs: live title/message/icon swap",
        args: &["--title", "Old", "--message", "Old body"],
        script: &[
            "title: Updated Title",
            "message: First line",
            "message: + appended line",
            "icon: SF=checkmark.circle.fill",
        ],
        check: |s| {
            assert_eq!(s.title.as_deref(), Some("Updated Title"));
            assert_eq!(s.message, "First line\n\nappended line");
            assert_eq!(s.icon.source, "SF=checkmark.circle.fill");
        },
    },
    ScriptCase {
        name: "list: replace then clear",
        args: &["--listitem", "Seed"],
        script: &["list: A, B, C", "listitem: title: B, status: success"],
        check: |s| {
            assert_eq!(
                s.list_items
                    .iter()
                    .map(|i| i.title.as_str())
                    .collect::<Vec<_>>(),
                ["A", "B", "C"]
            );
            assert_eq!(s.list_items[1].status, "success");
        },
    },
];

#[test]
fn command_file_contract() {
    for c in SCRIPT_CASES {
        println!("script case: {}", c.name);
        (c.check)(&run_script(c.args, c.script));
    }
}

// ── collected input → exit stdout ───────────────────────────────────────

#[test]
fn button1_plain_output_contract() {
    let out = button1_stdout(
        &[
            "--textfield",
            "Computer Name",
            "--selecttitle",
            "Department",
            "--selectvalues",
            "HR,Engineering,Operations",
            "--selectdefault",
            "Engineering",
            "--checkbox",
            "Enable FileVault",
        ],
        |s| {
            s.text_fields[0].value = "MBP-1234".into();
            s.checkboxes[0].checked = true;
        },
        false,
    );
    let lines: Vec<&str> = out.lines().collect();
    // textfield unquoted; legacy single-select keys; per-name select; checkbox quoted
    assert_eq!(lines[0], "Computer Name : MBP-1234");
    assert!(out.contains("\"SelectedOption\" : \"Engineering\""));
    assert!(out.contains("\"SelectedIndex\" : 1"));
    assert!(out.contains("\"Department\" : \"Engineering\""));
    assert!(out.contains("\"Department\" index : \"1\""));
    assert!(out.contains("\"Enable FileVault\" : \"true\""));
}

#[test]
fn button1_json_output_contract() {
    let json = button1_stdout(
        &[
            "--textfield",
            "Computer Name",
            "--selecttitle",
            "Department",
            "--selectvalues",
            "HR,Engineering,Operations",
            "--selectdefault",
            "Engineering",
        ],
        |s| s.text_fields[0].value = "MBP-1234".into(),
        true,
    );
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON stdout");
    assert_eq!(v["Computer Name"], "MBP-1234");
    assert_eq!(v["SelectedOption"], "Engineering");
    assert_eq!(v["SelectedIndex"], 1);
    assert_eq!(v["Department"]["selectedValue"], "Engineering");
    assert_eq!(v["Department"]["selectedIndex"], 1);
}

#[test]
fn non_button1_exit_withholds_inputs_by_default() {
    // A textfield value is NOT emitted on a button2/timer exit unless
    // --alwaysreturninput; list selections always are.
    let mut s = state(&[
        "--textfield",
        "Secret",
        "--enablelistselect",
        "--listitem",
        "Row",
    ]);
    s.text_fields[0].value = "hunter2".into();
    s.list_items[0].selected = true;
    let withheld = UserInput::from_dialog(&s, false);
    assert!(
        withheld.textfields.is_empty(),
        "textfields withheld off button1"
    );
    assert_eq!(withheld.list_selections, vec![("Row".to_string(), true)]);
}

// ── degrade-don't-break ─────────────────────────────────────────────────

#[test]
fn platform_unavailable_options_degrade() {
    // A macOS-only option on Windows is platform-unavailable even if listed
    // as implemented — the dialog still shows (no error).
    let implemented: HashSet<&str> = ["title", "notification"].into_iter().collect();
    let warnings = degrade_warnings(
        &parse(["--title", "Hi", "--notification"]),
        &implemented,
        Platform::Windows,
    );
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].option, "notification");
    assert_eq!(warnings[0].reason, DegradeReason::PlatformUnavailable);
}
