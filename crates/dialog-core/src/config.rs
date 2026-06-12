//! Dialog configuration: CLI arguments overlaid with JSON input from
//! `--jsonfile <path>` or `--jsonstring <json>`. JSON keys are the long
//! option names; structured elements (textfield, checkbox, listitem,
//! selectitems) stay as raw JSON values for their capability parsers.

use crate::exit_codes;
use crate::parser::ParsedArgs;
use serde_json::Value;
use std::borrow::Cow;

#[derive(Debug)]
pub enum JsonError {
    /// `--jsonfile` path unreadable → exit 202, upstream message.
    FileNotFound(String),
    /// Unparseable JSON → exit 202, "JSON import failed"
    /// (ProcessCLOptions.swift:28).
    Parse(String),
}

impl JsonError {
    pub fn exit_code(&self) -> i32 {
        exit_codes::FILE_NOT_FOUND
    }

    pub fn message(&self) -> String {
        match self {
            JsonError::FileNotFound(path) => {
                format!(
                    "{} {path}",
                    exit_codes::exit_message(exit_codes::FILE_NOT_FOUND).unwrap()
                )
            }
            JsonError::Parse(_) => "JSON import failed".to_string(),
        }
    }
}

/// Full dialog configuration: CLI args + optional JSON overlay.
/// Precedence: CLI value > JSON value > table default. (VERIFY-LIVE:
/// upstream merge precedence when both CLI and JSON set the same key.)
#[derive(Debug)]
pub struct Config {
    pub cli: ParsedArgs,
    pub json: Option<Value>,
}

impl Config {
    /// Build from parsed CLI args, loading JSON input if requested.
    pub fn load(cli: ParsedArgs) -> Result<Self, JsonError> {
        let json = if let Some(path) = cli.value("jsonfile") {
            let text = std::fs::read_to_string(path)
                .map_err(|_| JsonError::FileNotFound(path.to_string()))?;
            Some(serde_json::from_str(&text).map_err(|e| JsonError::Parse(e.to_string()))?)
        } else if let Some(s) = cli.value("jsonstring") {
            Some(serde_json::from_str(s).map_err(|e| JsonError::Parse(e.to_string()))?)
        } else {
            None
        };
        Ok(Config { cli, json })
    }

    fn json_field(&self, name: &str) -> Option<&Value> {
        self.json.as_ref()?.get(name)
    }

    /// Whether an option is set via CLI or JSON (truthy for JSON bools).
    pub fn present(&self, name: &str) -> bool {
        self.cli.present(name)
            || match self.json_field(name) {
                Some(Value::Bool(b)) => *b,
                Some(_) => true,
                None => false,
            }
    }

    /// Effective scalar value: CLI > JSON > table default.
    pub fn value(&self, name: &str) -> Option<Cow<'_, str>> {
        if let Some(v) = self.cli.value(name) {
            return Some(Cow::Borrowed(v));
        }
        match self.json_field(name) {
            Some(Value::String(s)) => Some(Cow::Borrowed(s.as_str())),
            Some(Value::Number(n)) => Some(Cow::Owned(n.to_string())),
            Some(Value::Bool(b)) => Some(Cow::Owned(b.to_string())),
            _ => crate::options::by_long(name)
                .and_then(|o| o.default)
                .map(Cow::Borrowed),
        }
    }

    /// Structured JSON array for an element key (e.g. "textfield",
    /// "checkbox", "listitem", "selectitems"), if provided via JSON.
    pub fn json_array(&self, name: &str) -> Option<&[Value]> {
        match self.json_field(name) {
            Some(Value::Array(a)) => Some(a.as_slice()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn config(args: &[&str]) -> Config {
        Config::load(parse(args.iter().copied())).unwrap()
    }

    #[test]
    fn jsonstring_drives_the_dialog() {
        let c = config(&[
            "--jsonstring",
            r#"{"title": "Hello", "message": "World", "button1text": "Done"}"#,
        ]);
        assert_eq!(c.value("title").unwrap(), "Hello");
        assert_eq!(c.value("message").unwrap(), "World");
        assert_eq!(c.value("button1text").unwrap(), "Done");
    }

    #[test]
    fn cli_wins_over_json() {
        let c = config(&["--title", "CLI", "--jsonstring", r#"{"title": "JSON"}"#]);
        assert_eq!(c.value("title").unwrap(), "CLI");
    }

    #[test]
    fn json_bool_flags_count_as_present() {
        let c = config(&["--jsonstring", r#"{"ontop": true, "blurscreen": false}"#]);
        assert!(c.present("ontop"));
        assert!(!c.present("blurscreen"));
    }

    #[test]
    fn json_numbers_stringify() {
        let c = config(&["--jsonstring", r#"{"width": 500}"#]);
        assert_eq!(c.value("width").unwrap(), "500");
    }

    #[test]
    fn defaults_still_apply_under_json() {
        let c = config(&["--jsonstring", r#"{"title": "X"}"#]);
        assert_eq!(c.value("button1text").unwrap(), "OK");
    }

    #[test]
    fn structured_arrays_pass_through() {
        let c = config(&[
            "--jsonstring",
            r#"{"textfield": [{"title": "Name", "required": true}]}"#,
        ]);
        let fields = c.json_array("textfield").unwrap();
        assert_eq!(fields[0]["title"], "Name");
    }

    #[test]
    fn missing_jsonfile_maps_to_exit_202() {
        let err = Config::load(parse(["--jsonfile", "/nonexistent.json"])).unwrap_err();
        assert_eq!(err.exit_code(), 202);
        assert_eq!(err.message(), "ERROR: File not found : /nonexistent.json");
    }

    #[test]
    fn bad_json_reports_import_failed() {
        let err = Config::load(parse(["--jsonstring", "{not json"])).unwrap_err();
        assert_eq!(err.exit_code(), 202);
        assert_eq!(err.message(), "JSON import failed");
    }
}
