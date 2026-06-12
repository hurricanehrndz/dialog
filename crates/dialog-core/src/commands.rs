//! Command-file verb parsing and application onto DialogState. Verbs and
//! semantics mirror `DialogUpdatableContent.swift` (v3.0.1). Lines that
//! parse to no known verb are ignored (command-file-ipc spec: malformed
//! commands MUST NOT stop processing).

use crate::state::{Alignment, DialogState, ProgressState, TimerState};

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressCmd {
    Set(f64),
    Increment(f64),
    Decrement(f64),
    /// reset / indeterminate
    Reset,
    Complete,
    /// delete/remove/hide/disable
    Hide,
    /// create/show/enable
    Show,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Title(String),
    Subtitle(String),
    Message { text: String, append: bool },
    Icon(String),
    IconAlpha(f64),
    IconSize(f64),
    Progress(ProgressCmd),
    ProgressText(String),
    Button1Text(String),
    Button2Text(String),
    Button1Enabled(bool),
    Button2Enabled(bool),
    InfoText(String),
    Alignment(Alignment),
    Width(f64),
    Height(f64),
    Position(String),
    QuitKey(String),
    Timer(TimerCmd),
    Quit,
    Activate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimerCmd {
    Set(f64),
    Hide,
}

/// Parse one command-file line. Returns None for blank/unknown/malformed
/// lines (callers log and continue).
pub fn parse_line(line: &str) -> Option<Command> {
    let line = line.trim();
    let (verb, arg) = line.split_once(':')?;
    let verb = verb.trim().to_lowercase();
    let arg = arg.trim();

    Some(match verb.as_str() {
        "title" => Command::Title(arg.to_string()),
        "subtitle" => Command::Subtitle(arg.to_string()),
        "message" => match arg.strip_prefix('+') {
            Some(rest) => Command::Message {
                text: rest.trim_start().to_string(),
                append: true,
            },
            None => Command::Message {
                text: arg.to_string(),
                append: false,
            },
        },
        "icon" => Command::Icon(arg.to_string()),
        "iconalpha" => Command::IconAlpha(arg.parse().ok()?),
        "iconsize" => Command::IconSize(arg.parse().ok()?),
        "progress" => Command::Progress(match arg.to_lowercase().as_str() {
            "increment" => ProgressCmd::Increment(1.0),
            "decrement" => ProgressCmd::Decrement(1.0),
            "reset" | "indeterminate" => ProgressCmd::Reset,
            "complete" => ProgressCmd::Complete,
            "delete" | "remove" | "hide" | "disable" => ProgressCmd::Hide,
            "create" | "show" | "enable" => ProgressCmd::Show,
            n => ProgressCmd::Set(n.parse().ok()?),
        }),
        "progresstext" => Command::ProgressText(arg.to_string()),
        "button1text" => Command::Button1Text(arg.to_string()),
        "button2text" => Command::Button2Text(arg.to_string()),
        "button1" => Command::Button1Enabled(parse_enabled(arg)?),
        "button2" => Command::Button2Enabled(parse_enabled(arg)?),
        "infotext" => Command::InfoText(arg.to_string()),
        "alignment" => Command::Alignment(Alignment::parse_str(arg)),
        "width" => Command::Width(arg.parse().ok()?),
        "height" => Command::Height(arg.parse().ok()?),
        "position" => Command::Position(arg.to_string()),
        "quitkey" => Command::QuitKey(arg.to_string()),
        "timer" => Command::Timer(match arg.to_lowercase().as_str() {
            "hide" => TimerCmd::Hide,
            n => TimerCmd::Set(n.parse().ok()?),
        }),
        "quit" => Command::Quit,
        "activate" => Command::Activate,
        _ => return None,
    })
}

fn parse_enabled(arg: &str) -> Option<bool> {
    match arg.to_lowercase().as_str() {
        "enable" => Some(true),
        "disable" => Some(false),
        _ => None,
    }
}

/// Mutate state per the command. Window-level commands (size/position/
/// activate/quit) are handled by the shell; this covers content state.
/// Returns false if the command needs shell-side handling.
pub fn apply(state: &mut DialogState, cmd: &Command) -> bool {
    match cmd {
        Command::Title(t) => {
            state.title = if t == "none" { None } else { Some(t.clone()) };
        }
        Command::Subtitle(t) => state.subtitle = Some(t.clone()),
        Command::Message { text, append } => {
            if *append {
                state.message.push_str("\n\n");
                state.message.push_str(text);
            } else {
                state.message = text.clone();
            }
        }
        Command::Icon(src) => {
            state.icon.source = src.clone();
            state.icon.hidden = src == "none";
        }
        Command::IconAlpha(a) => state.icon.alpha = *a,
        Command::IconSize(s) => state.icon.size = *s,
        Command::Progress(p) => {
            let progress = state.progress.get_or_insert(ProgressState {
                total: 100.0,
                current: None,
                text: String::new(),
                visible: true,
            });
            match p {
                ProgressCmd::Set(v) => progress.current = Some(*v),
                ProgressCmd::Increment(d) => {
                    progress.current =
                        Some((progress.current.unwrap_or(0.0) + d).min(progress.total))
                }
                ProgressCmd::Decrement(d) => {
                    progress.current = Some((progress.current.unwrap_or(0.0) - d).max(0.0))
                }
                ProgressCmd::Reset => progress.current = None,
                ProgressCmd::Complete => progress.current = Some(progress.total),
                ProgressCmd::Hide => progress.visible = false,
                ProgressCmd::Show => progress.visible = true,
            }
        }
        Command::ProgressText(t) => {
            let progress = state.progress.get_or_insert(ProgressState {
                total: 100.0,
                current: None,
                text: String::new(),
                visible: true,
            });
            progress.text = t.clone();
        }
        Command::Button1Text(t) => state.button1.text = t.clone(),
        Command::Button2Text(t) => state.button2.text = t.clone(),
        Command::Button1Enabled(e) => state.button1.enabled = *e,
        Command::Button2Enabled(e) => state.button2.enabled = *e,
        Command::InfoText(t) => state.info_text = Some(t.clone()),
        Command::Alignment(a) => state.message_alignment = *a,
        Command::QuitKey(k) => state.quit_key = k.clone(),
        Command::Timer(TimerCmd::Set(s)) => {
            state.timer = Some(TimerState {
                seconds: *s,
                hide_bar: state.timer.as_ref().is_some_and(|t| t.hide_bar),
            })
        }
        Command::Timer(TimerCmd::Hide) => state.timer = None,
        Command::Width(_)
        | Command::Height(_)
        | Command::Position(_)
        | Command::Quit
        | Command::Activate => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::parser::parse;

    fn fresh(args: &[&str]) -> DialogState {
        DialogState::from_config(&Config::load(parse(args.iter().copied())).unwrap())
    }

    #[test]
    fn title_and_message_update() {
        let mut s = fresh(&["--title", "Old", "--message", "old"]);
        apply(&mut s, &parse_line("title: New Title").unwrap());
        assert_eq!(s.title.as_deref(), Some("New Title"));
        apply(&mut s, &parse_line("message: Step one complete").unwrap());
        assert_eq!(s.message, "Step one complete");
        apply(&mut s, &parse_line("message: + And step two").unwrap());
        assert_eq!(s.message, "Step one complete\n\nAnd step two");
    }

    #[test]
    fn progress_lifecycle() {
        let mut s = fresh(&["--progress", "4"]);
        apply(&mut s, &parse_line("progress: increment").unwrap());
        apply(&mut s, &parse_line("progress: increment").unwrap());
        assert_eq!(s.progress.as_ref().unwrap().current, Some(2.0));
        apply(&mut s, &parse_line("progress: complete").unwrap());
        assert_eq!(s.progress.as_ref().unwrap().current, Some(4.0));
        apply(&mut s, &parse_line("progress: reset").unwrap());
        assert_eq!(s.progress.as_ref().unwrap().current, None);
        apply(
            &mut s,
            &parse_line("progresstext: Installing 3 of 7").unwrap(),
        );
        assert_eq!(s.progress.as_ref().unwrap().text, "Installing 3 of 7");
    }

    #[test]
    fn increment_clamps_to_total() {
        let mut s = fresh(&["--progress", "2"]);
        for _ in 0..5 {
            apply(&mut s, &parse_line("progress: increment").unwrap());
        }
        assert_eq!(s.progress.as_ref().unwrap().current, Some(2.0));
    }

    #[test]
    fn button_enable_disable() {
        let mut s = fresh(&["--button1disabled"]);
        assert!(!s.button1.enabled);
        apply(&mut s, &parse_line("button1: enable").unwrap());
        assert!(s.button1.enabled);
        apply(&mut s, &parse_line("button2: disable").unwrap());
        assert!(!s.button2.enabled);
    }

    #[test]
    fn icon_swap_and_none() {
        let mut s = fresh(&["--icon", "warning"]);
        apply(
            &mut s,
            &parse_line("icon: SF=checkmark.circle.fill").unwrap(),
        );
        assert_eq!(s.icon.source, "SF=checkmark.circle.fill");
        assert!(!s.icon.hidden);
        apply(&mut s, &parse_line("icon: none").unwrap());
        assert!(s.icon.hidden);
    }

    #[test]
    fn shell_commands_are_flagged() {
        let mut s = fresh(&[]);
        assert!(!apply(&mut s, &Command::Quit));
        assert!(!apply(&mut s, &parse_line("position: topright").unwrap()));
        assert!(!apply(&mut s, &parse_line("width: 900").unwrap()));
    }

    #[test]
    fn malformed_lines_yield_none() {
        assert_eq!(parse_line("this is not a command"), None);
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("frobnicate: yes"), None);
        assert_eq!(parse_line("progress: not-a-number"), None);
    }

    #[test]
    fn quit_parses() {
        assert_eq!(parse_line("quit:"), Some(Command::Quit));
    }
}
