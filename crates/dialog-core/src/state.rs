//! The declarative dialog state pushed to the renderer (design D2). The
//! webview renders exactly this model and emits semantic events back; all
//! contract logic stays on the Rust side. Serialized as camelCase JSON.

use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

impl Alignment {
    /// Parse swiftDialog's alignment values (`centre` accepted).
    pub fn parse_str(s: &str) -> Self {
        match s {
            "center" | "centre" => Alignment::Center,
            "right" => Alignment::Right,
            _ => Alignment::Left,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ButtonState {
    pub text: String,
    pub visible: bool,
    pub enabled: bool,
    pub action: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IconState {
    /// Raw `--icon` value; resolution to a concrete image happens in the
    /// icon pipeline (icon-branding capability).
    pub source: String,
    pub size: f64,
    pub alpha: f64,
    pub alt_text: String,
    pub overlay: Option<String>,
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    pub seconds: f64,
    pub hide_bar: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListItemState {
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    /// "" | wait | progress | success | fail | error | pending | none
    /// (ListView.swift:190-205).
    pub status: String,
    pub status_text: String,
    /// Per-row progress percent when status is "progress".
    pub progress: Option<f64>,
    /// Selection state under --enablelistselect.
    pub selected: bool,
}

impl ListItemState {
    pub fn new(title: impl Into<String>) -> Self {
        ListItemState {
            title: title.into(),
            subtitle: None,
            icon: None,
            status: String::new(),
            status_text: String::new(),
            progress: None,
            selected: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressState {
    /// Total steps (`--progress <n>`, default 100 when created by a verb).
    pub total: f64,
    /// None = indeterminate.
    pub current: Option<f64>,
    /// `--progresstext` / `progresstext:` line under the bar.
    pub text: String,
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    /// `--position` anchor (topleft..bottomright); None = centered.
    pub position: Option<String>,
    pub position_offset: f64,
    pub ontop: bool,
    pub moveable: bool,
    pub resizable: bool,
    /// "light" | "dark" | None (follow OS).
    pub appearance: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextFieldState {
    /// `name=` if given, else the title — the output key.
    pub name: String,
    pub title: String,
    pub value: String,
    pub prompt: Option<String>,
    pub secure: bool,
    pub required: bool,
    pub regex: Option<String>,
    pub regex_error: Option<String>,
    /// `confirm` field: value must match the named field (or the prior one).
    pub is_date: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckboxState {
    pub name: String,
    pub label: String,
    pub checked: bool,
    pub disabled: bool,
    /// "checkbox" | "switch" (from --checkboxstyle).
    pub style: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SelectState {
    pub name: String,
    pub title: String,
    pub values: Vec<String>,
    pub selected: String,
    pub required: bool,
    /// "dropdown" | "radio" (from --selectstyle or the `radio` flag).
    pub style: String,
}

/// Everything the renderer needs to draw the dialog. Tier 1 content;
/// grows as capabilities land.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogState {
    /// "macos" | "windows" | "linux" — drives the renderer's theme.
    pub platform: String,
    /// None = `--title none` (hide the title area).
    pub title: Option<String>,
    pub subtitle: Option<String>,
    /// Markdown source.
    pub message: String,
    pub message_alignment: Alignment,
    /// "top" | "centre"/"center" | "bottom"; renderer default = top.
    pub message_position: Option<String>,
    /// Raw `--titlefont` / `--messagefont` specs (`size=,colour=,...`).
    pub title_font: Option<String>,
    pub message_font: Option<String>,
    pub icon: IconState,
    pub button1: ButtonState,
    pub button2: ButtonState,
    pub info_button: ButtonState,
    pub timer: Option<TimerState>,
    pub list_items: Vec<ListItemState>,
    pub list_select_enabled: bool,
    /// `--liststyle` (compact | expanded).
    pub list_style: Option<String>,
    pub text_fields: Vec<TextFieldState>,
    pub checkboxes: Vec<CheckboxState>,
    pub selects: Vec<SelectState>,
    pub progress: Option<ProgressState>,
    /// `--infotext` / `infotext:` — small text in the bottom-left.
    pub info_text: Option<String>,
    pub window: WindowState,
    /// Compact 540×128 layout (`--mini` or `--style mini`).
    pub mini: bool,
    /// `--style` preset (centred/alert/caution/warning/presentation/...).
    pub style: Option<String>,
    /// Quit key character (Cmd/Ctrl+<key> exits 10). Default "q".
    pub quit_key: String,
}

fn opt_value(config: &Config, name: &str) -> Option<String> {
    config.value(name).map(|v| v.into_owned())
}

/// Textfields from repeated `--textfield` specs and/or the JSON array.
/// Spec: `"Title,required,secure,prompt=..,value=..,regex=..,regexerror=..,name=.."`
fn parse_text_fields(config: &Config) -> Vec<TextFieldState> {
    use crate::suboptions::parse_spec;
    let mut fields: Vec<TextFieldState> = config
        .cli
        .values("textfield")
        .iter()
        .map(|spec| {
            let s = parse_spec(spec);
            TextFieldState {
                name: s.get("name").unwrap_or(&s.title).to_string(),
                title: s.title.clone(),
                value: s.get("value").unwrap_or_default().to_string(),
                prompt: s.get("prompt").map(str::to_string),
                secure: s.flag("secure"),
                required: s.flag("required"),
                regex: s.get("regex").map(str::to_string),
                regex_error: s.get("regexerror").map(str::to_string),
                is_date: s.flag("isdate"),
            }
        })
        .collect();
    if let Some(arr) = config.json_array("textfield") {
        for v in arr {
            if let Some(f) = textfield_from_json(v) {
                fields.push(f);
            }
        }
    }
    fields
}

fn textfield_from_json(v: &serde_json::Value) -> Option<TextFieldState> {
    match v {
        serde_json::Value::String(title) => Some(TextFieldState {
            name: title.clone(),
            title: title.clone(),
            value: String::new(),
            prompt: None,
            secure: false,
            required: false,
            regex: None,
            regex_error: None,
            is_date: false,
        }),
        serde_json::Value::Object(o) => {
            let s = |k: &str| o.get(k).and_then(|x| x.as_str()).map(str::to_string);
            let b = |k: &str| o.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
            let title = s("title")?;
            Some(TextFieldState {
                name: s("name").unwrap_or_else(|| title.clone()),
                title,
                value: s("value").unwrap_or_default(),
                prompt: s("prompt"),
                secure: b("secure"),
                required: b("required"),
                regex: s("regex"),
                regex_error: s("regexerror"),
                is_date: b("isdate"),
            })
        }
        _ => None,
    }
}

/// Checkboxes from repeated `--checkbox` specs and/or JSON array.
fn parse_checkboxes(config: &Config) -> Vec<CheckboxState> {
    use crate::suboptions::parse_spec;
    let global_style = config
        .value("checkboxstyle")
        .map(|v| v.into_owned())
        .unwrap_or_else(|| "checkbox".into());
    let mut boxes: Vec<CheckboxState> = config
        .cli
        .values("checkbox")
        .iter()
        .map(|spec| {
            let s = parse_spec(spec);
            CheckboxState {
                name: s.get("name").unwrap_or(&s.title).to_string(),
                label: s.title.clone(),
                checked: s.flag("checked"),
                disabled: s.flag("disabled"),
                style: global_style.clone(),
            }
        })
        .collect();
    if let Some(arr) = config.json_array("checkbox") {
        for v in arr {
            if let serde_json::Value::Object(o) = v {
                let s = |k: &str| o.get(k).and_then(|x| x.as_str()).map(str::to_string);
                let b = |k: &str| o.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
                if let Some(label) = s("label") {
                    boxes.push(CheckboxState {
                        name: s("name").unwrap_or_else(|| label.clone()),
                        label,
                        checked: b("checked"),
                        disabled: b("disabled"),
                        style: global_style.clone(),
                    });
                }
            }
        }
    }
    boxes
}

/// Selects from paired `--selecttitle`/`--selectvalues`/`--selectdefault`
/// (positionally matched) and/or the JSON `selectitems` array.
fn parse_selects(config: &Config) -> Vec<SelectState> {
    use crate::suboptions::parse_spec;
    let titles = config.cli.values("selecttitle");
    let values = config.cli.values("selectvalues");
    let defaults = config.cli.values("selectdefault");
    let styles = config.cli.values("selectstyle");
    let mut selects = Vec::new();
    for (i, title_spec) in titles.iter().enumerate() {
        let s = parse_spec(title_spec);
        let vals: Vec<String> = values
            .get(i)
            .map(|v| v.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default();
        let style = styles
            .get(i)
            .cloned()
            .or_else(|| s.flag("radio").then(|| "radio".to_string()))
            .unwrap_or_else(|| "dropdown".to_string());
        selects.push(SelectState {
            name: s.get("name").unwrap_or(&s.title).to_string(),
            title: s.title.clone(),
            values: vals,
            selected: defaults.get(i).cloned().unwrap_or_default(),
            required: s.flag("required"),
            style,
        });
    }
    if let Some(arr) = config.json_array("selectitems") {
        for v in arr {
            if let serde_json::Value::Object(o) = v {
                let title = o.get("title").and_then(|x| x.as_str()).unwrap_or("");
                if title.is_empty() {
                    continue;
                }
                let vals = o
                    .get("values")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                selects.push(SelectState {
                    name: o
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or(title)
                        .to_string(),
                    title: title.to_string(),
                    values: vals,
                    selected: o
                        .get("default")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    required: o.get("required").and_then(|x| x.as_bool()).unwrap_or(false),
                    style: o
                        .get("style")
                        .and_then(|x| x.as_str())
                        .unwrap_or("dropdown")
                        .to_string(),
                });
            }
        }
    }
    selects
}

/// List items from repeated `--listitem` specs and/or the JSON `listitem`
/// array (strings or {title,subtitle,icon,status,statustext} objects).
fn parse_list_items(config: &Config) -> Vec<ListItemState> {
    let mut items: Vec<ListItemState> = config
        .cli
        .values("listitem")
        .iter()
        .map(|spec| {
            let sub = crate::suboptions::parse_spec(spec);
            let mut item = ListItemState::new(sub.title.clone());
            item.subtitle = sub.get("subtitle").map(str::to_string);
            item.icon = sub.get("icon").map(str::to_string);
            item.status = sub.get("status").unwrap_or_default().to_string();
            item.status_text = sub.get("statustext").unwrap_or_default().to_string();
            item
        })
        .collect();

    if let Some(json_items) = config.json_array("listitem") {
        for v in json_items {
            let mut item = match v {
                serde_json::Value::String(s) => ListItemState::new(s.clone()),
                serde_json::Value::Object(o) => {
                    let get = |k: &str| o.get(k).and_then(|x| x.as_str()).map(str::to_string);
                    let mut i = ListItemState::new(get("title").unwrap_or_default());
                    i.subtitle = get("subtitle");
                    i.icon = get("icon");
                    i.status = get("status").unwrap_or_default();
                    i.status_text = get("statustext").unwrap_or_default();
                    i
                }
                _ => continue,
            };
            if item.title.is_empty() {
                continue;
            }
            item.selected = false;
            items.push(item);
        }
    }
    items
}

fn parse_f64(config: &Config, name: &str, fallback: f64) -> f64 {
    config
        .value(name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

impl DialogState {
    /// Translate parsed configuration into the initial render state.
    pub fn from_config(config: &Config) -> Self {
        let title = match config.value("title").as_deref() {
            Some("none") => None,
            Some(t) => Some(t.to_string()),
            None => None,
        };

        let style = opt_value(config, "style");
        let mini = config.present("mini") || style.as_deref() == Some("mini");

        // --small/--big scale the *default* window size (ProcessCLOptions
        // scaleFactor 0.75/1.25); explicit --width/--height win unscaled;
        // mini forces 540×128 (dialogApp.swift:236-239).
        let scale = if config.present("small") {
            0.75
        } else if config.present("big") {
            1.25
        } else {
            1.0
        };
        let (width, height) = if mini {
            (540.0, 128.0)
        } else {
            (
                if config.present("width") {
                    parse_f64(config, "width", 820.0)
                } else {
                    820.0 * scale
                },
                if config.present("height") {
                    parse_f64(config, "height", 380.0)
                } else {
                    380.0 * scale
                },
            )
        };

        // button2 appears with --button2 or --button2text;
        // the info button with --infobutton or --infobuttontext.
        let button2_visible = config.present("button2") || config.cli.present("button2text");
        let info_visible = config.present("infobutton") || config.cli.present("infobuttontext");

        let timer = config.present("timer").then(|| TimerState {
            seconds: parse_f64(config, "timer", 10.0),
            hide_bar: config.present("hidetimerbar"),
        });

        DialogState {
            platform: match crate::compat::Platform::current() {
                crate::compat::Platform::MacOs => "macos",
                crate::compat::Platform::Windows => "windows",
                crate::compat::Platform::Linux => "linux",
            }
            .to_string(),
            title,
            subtitle: opt_value(config, "subtitle"),
            message: config
                .value("message")
                .map(|v| v.into_owned())
                .unwrap_or_default(),
            message_alignment: Alignment::parse_str(
                config
                    .value("messagealignment")
                    .as_deref()
                    .unwrap_or("left"),
            ),
            message_position: opt_value(config, "messageposition"),
            title_font: opt_value(config, "titlefont"),
            message_font: opt_value(config, "messagefont"),
            icon: IconState {
                source: config
                    .value("icon")
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|| "default".into()),
                size: parse_f64(config, "iconsize", 150.0),
                alpha: parse_f64(config, "iconalpha", 1.0),
                alt_text: config
                    .value("iconalttext")
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|| "Dialog Icon".into()),
                overlay: opt_value(config, "overlayicon"),
                hidden: config.present("hideicon"),
            },
            button1: ButtonState {
                text: config
                    .value("button1text")
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|| "OK".into()),
                visible: true,
                enabled: !config.present("button1disabled"),
                action: opt_value(config, "button1action"),
            },
            button2: ButtonState {
                text: config
                    .value("button2text")
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|| "Cancel".into()),
                visible: button2_visible,
                enabled: !config.present("button2disabled"),
                action: opt_value(config, "button2action"),
            },
            info_button: ButtonState {
                text: config
                    .value("infobuttontext")
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|| "More Information".into()),
                visible: info_visible,
                enabled: true,
                action: opt_value(config, "infobuttonaction"),
            },
            timer,
            list_items: parse_list_items(config),
            list_select_enabled: config.present("enablelistselect"),
            list_style: opt_value(config, "liststyle"),
            text_fields: parse_text_fields(config),
            checkboxes: parse_checkboxes(config),
            selects: parse_selects(config),
            progress: config.present("progress").then(|| ProgressState {
                total: config
                    .value("progress")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100.0),
                // no numeric value -> indeterminate (list-progress spec)
                current: None,
                text: config
                    .value("progresstext")
                    .map(|v| v.into_owned())
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                visible: true,
            }),
            info_text: config.cli.present("infotext").then(|| {
                config
                    .value("infotext")
                    .map(|v| v.into_owned())
                    .unwrap_or_default()
            }),
            mini,
            style,
            quit_key: config
                .value("quitkey")
                .map(|v| v.into_owned())
                .unwrap_or_else(|| "q".into()),
            window: WindowState {
                width,
                height,
                position: opt_value(config, "position"),
                position_offset: parse_f64(config, "positionoffset", 16.0),
                ontop: config.present("ontop"),
                moveable: config.present("moveable"),
                resizable: config.present("resizable"),
                appearance: opt_value(config, "appearance"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn state(args: &[&str]) -> DialogState {
        DialogState::from_config(&Config::load(parse(args.iter().copied())).unwrap())
    }

    #[test]
    fn defaults_match_upstream() {
        let s = state(&["--message", "hi"]);
        assert_eq!(s.window.width, 820.0);
        assert_eq!(s.window.height, 380.0);
        assert_eq!(s.button1.text, "OK");
        assert!(s.button1.visible && s.button1.enabled);
        assert!(!s.button2.visible);
        assert!(!s.info_button.visible);
        assert_eq!(s.icon.size, 150.0);
        assert!(s.timer.is_none());
    }

    #[test]
    fn title_none_hides_title() {
        assert_eq!(state(&["--title", "none"]).title, None);
        assert_eq!(state(&["--title", "Hi"]).title.as_deref(), Some("Hi"));
    }

    #[test]
    fn button2text_implies_visibility() {
        let s = state(&["--button2text", "Later"]);
        assert!(s.button2.visible);
        assert_eq!(s.button2.text, "Later");
    }

    #[test]
    fn timer_defaults_to_ten_seconds() {
        let s = state(&["--timer"]);
        assert_eq!(s.timer.as_ref().unwrap().seconds, 10.0);
        let s = state(&["--timer", "30", "--hidetimerbar"]);
        let t = s.timer.unwrap();
        assert_eq!(t.seconds, 30.0);
        assert!(t.hide_bar);
    }

    #[test]
    fn centre_spelling_accepted() {
        let s = state(&["--messagealignment", "centre"]);
        assert_eq!(s.message_alignment, Alignment::Center);
    }

    #[test]
    fn small_and_big_scale_default_window() {
        let s = state(&["--small"]);
        assert_eq!((s.window.width, s.window.height), (615.0, 285.0));
        let s = state(&["--big"]);
        assert_eq!((s.window.width, s.window.height), (1025.0, 475.0));
    }

    #[test]
    fn explicit_size_wins_over_scale() {
        let s = state(&["--small", "--width", "700"]);
        assert_eq!(s.window.width, 700.0);
        assert_eq!(s.window.height, 285.0);
    }

    #[test]
    fn mini_forces_540_by_128() {
        for args in [&["--mini"][..], &["--style", "mini"][..]] {
            let s = state(args);
            assert!(s.mini);
            assert_eq!((s.window.width, s.window.height), (540.0, 128.0));
        }
    }

    #[test]
    fn quit_key_default_and_override() {
        assert_eq!(state(&["--message", "x"]).quit_key, "q");
        assert_eq!(state(&["--quitkey", "x"]).quit_key, "x");
    }

    #[test]
    fn json_config_feeds_state() {
        let s = state(&[
            "--jsonstring",
            r#"{"title": "Hello", "ontop": true, "width": 500}"#,
        ]);
        assert_eq!(s.title.as_deref(), Some("Hello"));
        assert!(s.window.ontop);
        assert_eq!(s.window.width, 500.0);
    }

    #[test]
    fn serializes_camel_case() {
        let v = serde_json::to_value(state(&["--message", "x"])).unwrap();
        assert!(v.get("messageAlignment").is_some());
        assert!(v["window"].get("positionOffset").is_some());
        assert!(v["button1"].get("enabled").is_some());
    }
}
