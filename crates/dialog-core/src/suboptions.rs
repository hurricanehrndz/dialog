//! swiftDialog's comma-separated sub-option syntax, shared by --listitem,
//! --textfield, --checkbox, --selecttitle:
//! `"Title,key=value,flag,key2=value2"` — first comma-free-of-`=` segment
//! is the title; `key=value` pairs and bare flags follow.

use std::collections::HashMap;

#[derive(Debug, Default, PartialEq)]
pub struct SubOptions {
    pub title: String,
    pub values: HashMap<String, String>,
    pub flags: Vec<String>,
}

pub fn parse_spec(spec: &str) -> SubOptions {
    let mut out = SubOptions::default();
    for (i, part) in spec.split(',').enumerate() {
        let part = part.trim();
        match part.split_once('=') {
            Some((k, v)) => {
                out.values
                    .insert(k.trim().to_lowercase(), v.trim().to_string());
            }
            None if i == 0 => out.title = part.to_string(),
            None if !part.is_empty() => out.flags.push(part.to_lowercase()),
            None => {}
        }
    }
    out
}

impl SubOptions {
    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_flags_and_values() {
        let s = parse_spec("Serial Number,required,secure,prompt=Enter serial,regex=^[A-Z0-9]+$");
        assert_eq!(s.title, "Serial Number");
        assert!(s.flag("required") && s.flag("secure"));
        assert_eq!(s.get("prompt"), Some("Enter serial"));
        assert_eq!(s.get("regex"), Some("^[A-Z0-9]+$"));
    }

    #[test]
    fn plain_title_only() {
        let s = parse_spec("Install Chrome");
        assert_eq!(s.title, "Install Chrome");
        assert!(s.flags.is_empty() && s.values.is_empty());
    }

    #[test]
    fn listitem_style_spec() {
        let s = parse_spec("Enroll Device,status=wait,statustext=Working");
        assert_eq!(s.title, "Enroll Device");
        assert_eq!(s.get("status"), Some("wait"));
        assert_eq!(s.get("statustext"), Some("Working"));
    }
}
