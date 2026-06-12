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
    fn parse(s: &str) -> Self {
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

/// Everything the renderer needs to draw the dialog. Tier 1 content;
/// grows as capabilities land.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialogState {
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
    pub window: WindowState,
}

fn opt_value(config: &Config, name: &str) -> Option<String> {
    config.value(name).map(|v| v.into_owned())
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

        // button2 appears with --button2 or --button2text;
        // the info button with --infobutton or --infobuttontext.
        let button2_visible = config.present("button2") || config.cli.present("button2text");
        let info_visible = config.present("infobutton") || config.cli.present("infobuttontext");

        let timer = config.present("timer").then(|| TimerState {
            seconds: parse_f64(config, "timer", 10.0),
            hide_bar: config.present("hidetimerbar"),
        });

        DialogState {
            title,
            subtitle: opt_value(config, "subtitle"),
            message: config
                .value("message")
                .map(|v| v.into_owned())
                .unwrap_or_default(),
            message_alignment: Alignment::parse(
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
            window: WindowState {
                width: parse_f64(config, "width", 820.0),
                height: parse_f64(config, "height", 380.0),
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
