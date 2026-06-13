//! Command-file verb parsing and application onto DialogState. Verbs and
//! semantics mirror `DialogUpdatableContent.swift` (v3.0.1). Lines that
//! parse to no known verb are ignored (command-file-ipc spec: malformed
//! commands MUST NOT stop processing).

use crate::state::{Alignment, DialogState, ListItemState, ProgressState, TimerState};

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
    Message {
        text: String,
        append: bool,
    },
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
    ListItem(ListItemUpdate),
    /// `list: a,b,c` replaces all rows; `list: clear` empties.
    ListReplace(Vec<String>),
    Quit,
    Activate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListAction {
    #[default]
    Update,
    Add,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListItemUpdate {
    pub action: ListAction,
    /// Row addressed by index (`index: <n>`) ...
    pub index: Option<usize>,
    /// ... or by title (`title: <t>`; also the new title for `add`).
    pub title: Option<String>,
    pub status: Option<String>,
    pub status_text: Option<String>,
    pub progress: Option<f64>,
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
        "list" => match arg.to_lowercase().as_str() {
            "clear" => Command::ListReplace(Vec::new()),
            _ => Command::ListReplace(
                arg.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            ),
        },
        "listitem" => Command::ListItem(parse_listitem(arg)?),
        _ => return None,
    })
}

/// `listitem:` argument forms (DialogUpdatableContent.swift):
/// keyed — `title: X, status: success, statustext: Done`,
///         `index: 2, progress: 50`, `add, title: X, status: wait`,
///         `delete, title: X` / `delete, index: 0`;
/// legacy — `<title>: <status>` (split at the last colon).
fn parse_listitem(arg: &str) -> Option<ListItemUpdate> {
    let mut upd = ListItemUpdate::default();
    let mut keyed = false;
    for part in arg.split(',') {
        let part = part.trim();
        match part.split_once(':') {
            Some((k, v)) => {
                let v = v.trim();
                keyed = true;
                match k.trim().to_lowercase().as_str() {
                    "title" => upd.title = Some(v.to_string()),
                    "index" => upd.index = v.parse().ok(),
                    "status" => upd.status = Some(v.to_string()),
                    "statustext" => upd.status_text = Some(v.to_string()),
                    "progress" => upd.progress = v.parse().ok(),
                    _ => keyed = false,
                }
            }
            None => match part.to_lowercase().as_str() {
                "add" => upd.action = ListAction::Add,
                "delete" => upd.action = ListAction::Delete,
                _ => {}
            },
        }
    }
    if !keyed && upd.action == ListAction::Update {
        // legacy form "<title>: <status>"
        let (title, status) = arg.rsplit_once(':')?;
        upd.title = Some(title.trim().to_string());
        upd.status = Some(status.trim().to_string());
    }
    (upd.title.is_some() || upd.index.is_some()).then_some(upd)
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
        Command::ListReplace(titles) => {
            state.list_items = titles.iter().map(ListItemState::new).collect();
        }
        Command::ListItem(upd) => match upd.action {
            ListAction::Add => {
                let mut item = ListItemState::new(upd.title.clone().unwrap_or_default());
                item.status = upd.status.clone().unwrap_or_default();
                item.status_text = upd.status_text.clone().unwrap_or_default();
                item.progress = upd.progress;
                state.list_items.push(item);
            }
            ListAction::Delete => {
                if let Some(i) = upd.index {
                    if i < state.list_items.len() {
                        state.list_items.remove(i);
                    }
                } else if let Some(t) = &upd.title {
                    state.list_items.retain(|item| &item.title != t);
                }
            }
            ListAction::Update => {
                let item = match (upd.index, &upd.title) {
                    (Some(i), _) => state.list_items.get_mut(i),
                    (None, Some(t)) => state.list_items.iter_mut().find(|item| &item.title == t),
                    _ => None,
                };
                if let Some(item) = item {
                    if let Some(s) = &upd.status {
                        item.status = s.clone();
                    }
                    if let Some(t) = &upd.status_text {
                        item.status_text = t.clone();
                    }
                    if let Some(p) = upd.progress {
                        item.progress = Some(p);
                        item.status = "progress".into();
                    }
                }
            }
        },
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

    #[test]
    fn listitem_update_by_title_and_index() {
        let mut s = fresh(&["--listitem", "Install Chrome", "--listitem", "Enroll"]);
        apply(
            &mut s,
            &parse_line("listitem: title: Install Chrome, status: success, statustext: Done")
                .unwrap(),
        );
        assert_eq!(s.list_items[0].status, "success");
        assert_eq!(s.list_items[0].status_text, "Done");
        apply(
            &mut s,
            &parse_line("listitem: index: 1, progress: 40").unwrap(),
        );
        assert_eq!(s.list_items[1].progress, Some(40.0));
        assert_eq!(s.list_items[1].status, "progress");
    }

    #[test]
    fn listitem_legacy_form() {
        let mut s = fresh(&["--listitem", "Install Chrome"]);
        apply(
            &mut s,
            &parse_line("listitem: Install Chrome: wait").unwrap(),
        );
        assert_eq!(s.list_items[0].status, "wait");
    }

    #[test]
    fn listitem_add_and_delete() {
        let mut s = fresh(&["--listitem", "A"]);
        apply(
            &mut s,
            &parse_line("listitem: add, title: B, status: pending").unwrap(),
        );
        assert_eq!(s.list_items.len(), 2);
        assert_eq!(s.list_items[1].title, "B");
        apply(&mut s, &parse_line("listitem: delete, title: A").unwrap());
        assert_eq!(s.list_items.len(), 1);
        assert_eq!(s.list_items[0].title, "B");
    }

    #[test]
    fn list_replace_and_clear() {
        let mut s = fresh(&["--listitem", "Old"]);
        apply(&mut s, &parse_line("list: Step 1, Step 2, Step 3").unwrap());
        assert_eq!(
            s.list_items
                .iter()
                .map(|i| i.title.as_str())
                .collect::<Vec<_>>(),
            ["Step 1", "Step 2", "Step 3"]
        );
        apply(&mut s, &parse_line("list: clear").unwrap());
        assert!(s.list_items.is_empty());
    }

    #[test]
    fn listitem_spec_parsing_from_cli() {
        let s = fresh(&[
            "--listitem",
            "Enroll Device,status=wait,statustext=Working,subtitle=MDM",
        ]);
        let item = &s.list_items[0];
        assert_eq!(item.title, "Enroll Device");
        assert_eq!(item.status, "wait");
        assert_eq!(item.status_text, "Working");
        assert_eq!(item.subtitle.as_deref(), Some("MDM"));
    }
}
