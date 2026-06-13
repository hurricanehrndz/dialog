//! Exit output contract: the `key : value` lines and `--json` object that
//! scripts parse. Formats are bit-for-bit from `Functions/System.swift`
//! (v3.0.1) — note the quoting quirks: textfield lines are unquoted,
//! select/checkbox/list lines are quoted, plain `SelectedIndex` is a bare
//! number.

use crate::state::DialogState;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone)]
pub struct SelectResult {
    /// `name=` sub-option if given, else the select title.
    pub name: String,
    pub selected_value: String,
    /// Index into the values list, -1 if unmatched.
    pub selected_index: i64,
}

/// Everything user input contributes to exit output, in upstream's
/// emission order: textfields → selects → list selections → checkboxes.
#[derive(Debug, Clone, Default)]
pub struct UserInput {
    pub textfields: Vec<(String, String)>,
    pub selects: Vec<SelectResult>,
    pub list_selections: Vec<(String, bool)>,
    pub checkboxes: Vec<(String, bool)>,
}

impl UserInput {
    /// Collect the output an exit emits from the dialog state — the exact
    /// mapping the app uses on quit. List selections are included whenever
    /// `--enablelistselect` is set; textfields/checkboxes/selects only when
    /// `emit_inputs` (button1, or any exit under `--alwaysreturninput`).
    pub fn from_dialog(state: &DialogState, emit_inputs: bool) -> Self {
        let mut input = UserInput::default();
        if state.list_select_enabled {
            input.list_selections = state
                .list_items
                .iter()
                .map(|i| (i.title.clone(), i.selected))
                .collect();
        }
        if emit_inputs {
            input.textfields = state
                .text_fields
                .iter()
                .map(|f| (f.name.clone(), f.value.clone()))
                .collect();
            input.checkboxes = state
                .checkboxes
                .iter()
                .map(|c| (c.name.clone(), c.checked))
                .collect();
            input.selects = state
                .selects
                .iter()
                .map(|s| {
                    let selected_index = s
                        .values
                        .iter()
                        .position(|v| v == &s.selected)
                        .map(|i| i as i64)
                        .unwrap_or(-1);
                    SelectResult {
                        name: s.name.clone(),
                        selected_value: s.selected.clone(),
                        selected_index,
                    }
                })
                .collect();
        }
        input
    }

    pub fn is_empty(&self) -> bool {
        self.textfields.is_empty()
            && self.selects.is_empty()
            && self.list_selections.is_empty()
            && self.checkboxes.is_empty()
    }

    /// Default (non-`--json`) output lines, exactly as upstream appends to
    /// `outputArray`.
    pub fn plain_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, value) in &self.textfields {
            out.push(format!("{name} : {value}"));
        }
        // Single-select dialogs also emit legacy keys (System.swift:307-310).
        if self.selects.len() == 1 {
            let s = &self.selects[0];
            out.push(format!("\"SelectedOption\" : \"{}\"", s.selected_value));
            out.push(format!("\"SelectedIndex\" : {}", s.selected_index));
        }
        for s in &self.selects {
            out.push(format!("\"{}\" : \"{}\"", s.name, s.selected_value));
            out.push(format!("\"{}\" index : \"{}\"", s.name, s.selected_index));
        }
        for (title, selected) in &self.list_selections {
            out.push(format!("\"{title}\" : \"{selected}\""));
        }
        for (name, checked) in &self.checkboxes {
            out.push(format!("\"{name}\" : \"{checked}\""));
        }
        out
    }

    /// `--json` output object.
    pub fn json_object(&self) -> Value {
        let mut map = Map::new();
        for (name, value) in &self.textfields {
            map.insert(name.clone(), Value::String(value.clone()));
        }
        if self.selects.len() == 1 {
            let s = &self.selects[0];
            map.insert(
                "SelectedOption".into(),
                Value::String(s.selected_value.clone()),
            );
            map.insert("SelectedIndex".into(), json!(s.selected_index));
        }
        for s in &self.selects {
            map.insert(
                s.name.clone(),
                json!({
                    "selectedValue": s.selected_value,
                    "selectedIndex": s.selected_index,
                }),
            );
        }
        for (title, selected) in &self.list_selections {
            map.insert(title.clone(), Value::Bool(*selected));
        }
        for (name, checked) in &self.checkboxes {
            map.insert(name.clone(), Value::Bool(*checked));
        }
        Value::Object(map)
    }

    /// Render the final stdout payload. (VERIFY-LIVE: upstream prints
    /// SwiftyJSON's pretty format; confirm indentation against the real
    /// binary in the compat harness.)
    pub fn render(&self, as_json: bool) -> String {
        if as_json {
            serde_json::to_string_pretty(&self.json_object()).expect("valid json")
        } else {
            self.plain_lines().join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> UserInput {
        UserInput {
            textfields: vec![("Computer Name".into(), "kraken".into())],
            selects: vec![SelectResult {
                name: "Dept".into(),
                selected_value: "HR".into(),
                selected_index: 1,
            }],
            list_selections: vec![("Optional App A".into(), true)],
            checkboxes: vec![("Enable FileVault".into(), false)],
        }
    }

    #[test]
    fn plain_lines_match_upstream_formats() {
        assert_eq!(
            sample().plain_lines(),
            vec![
                "Computer Name : kraken",           // unquoted (textfield)
                "\"SelectedOption\" : \"HR\"",      // legacy single-select
                "\"SelectedIndex\" : 1",            // bare number
                "\"Dept\" : \"HR\"",                // per-name select
                "\"Dept\" index : \"1\"",           // quoted index
                "\"Optional App A\" : \"true\"",    // list selection
                "\"Enable FileVault\" : \"false\"", // checkbox
            ]
        );
    }

    #[test]
    fn json_object_matches_upstream_shape() {
        let v = sample().json_object();
        assert_eq!(v["Computer Name"], "kraken");
        assert_eq!(v["SelectedOption"], "HR");
        assert_eq!(v["SelectedIndex"], 1);
        assert_eq!(v["Dept"]["selectedValue"], "HR");
        assert_eq!(v["Dept"]["selectedIndex"], 1);
        assert_eq!(v["Optional App A"], true);
        assert_eq!(v["Enable FileVault"], false);
    }

    #[test]
    fn multiple_selects_skip_legacy_keys() {
        let mut s = sample();
        s.selects.push(SelectResult {
            name: "Site".into(),
            selected_value: "NYC".into(),
            selected_index: 0,
        });
        let v = s.json_object();
        assert!(v.get("SelectedOption").is_none());
        assert!(!s.plain_lines().iter().any(|l| l.contains("SelectedOption")));
    }

    #[test]
    fn unmatched_select_reports_minus_one() {
        let s = UserInput {
            selects: vec![SelectResult {
                name: "X".into(),
                selected_value: String::new(),
                selected_index: -1,
            }],
            ..Default::default()
        };
        assert_eq!(s.json_object()["X"]["selectedIndex"], -1);
    }

    #[test]
    fn no_input_renders_empty() {
        assert!(UserInput::default().is_empty());
        assert_eq!(UserInput::default().render(false), "");
    }
}
