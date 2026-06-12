//! CLI argument parsing over the option table. Accepts the full swiftDialog
//! surface: `--long` and `-short` forms, deprecated aliases resolved to
//! canonical names, repeatable options accumulated in order.

use crate::options::{self, Kind, OptionDef};
use std::collections::HashMap;

/// Parsed command line, keyed by canonical long option names.
#[derive(Debug, Default)]
pub struct ParsedArgs {
    /// Values for value-taking options, in command-line order.
    /// Non-repeatable options still record every occurrence; `value()`
    /// returns the last one (matching swiftDialog's scan behavior).
    values: HashMap<&'static str, Vec<String>>,
    /// Flags that were present.
    flags: Vec<&'static str>,
    /// Tokens that matched no option — ignored like upstream does;
    /// surfaced on stderr only under `--verbose`.
    pub ignored: Vec<String>,
}

impl ParsedArgs {
    /// Whether an option (flag or value) appeared on the command line.
    pub fn present(&self, long: &str) -> bool {
        // not `contains`: &str vs &'static str lifetimes don't unify there
        #[allow(clippy::manual_contains)]
        let in_flags = self.flags.iter().any(|&f| f == long);
        in_flags || self.values.contains_key(long)
    }

    /// Last value given for an option, if any.
    pub fn value(&self, long: &str) -> Option<&str> {
        self.values.get(long)?.last().map(String::as_str)
    }

    /// Last value given, falling back to the table default.
    pub fn value_or_default(&self, long: &str) -> Option<&str> {
        self.value(long).or_else(|| options::by_long(long)?.default)
    }

    /// All values for a repeatable option, in order.
    pub fn values(&self, long: &str) -> &[String] {
        self.values.get(long).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Canonical long names of everything present, for the
    /// degrade-don't-break warning pass.
    pub fn present_options(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.flags
            .iter()
            .copied()
            .chain(self.values.keys().copied())
    }
}

fn lookup(arg: &str) -> Option<&'static OptionDef> {
    if let Some(long) = arg.strip_prefix("--") {
        options::by_long(long)
    } else if let Some(short) = arg.strip_prefix('-') {
        options::by_short(short)
    } else {
        None
    }
}

/// Parse argv (without the program name).
///
/// Tokens that match nothing in the option table are recorded in
/// `ignored` and skipped — upstream scans argv for known options and
/// never errors on strays (verified live against swiftDialog 3.0.1).
pub fn parse<I, S>(args: I) -> ParsedArgs
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut parsed = ParsedArgs::default();
    let mut iter = args.into_iter().peekable();

    while let Some(arg) = iter.next() {
        let arg = arg.as_ref();
        let Some(def) = lookup(arg) else {
            parsed.ignored.push(arg.to_string());
            continue;
        };
        let canonical = options::canonical(def);
        match canonical.kind {
            Kind::Flag => parsed.flags.push(canonical.long),
            Kind::Value => {
                // swiftDialog consumes the next argument unconditionally as
                // the value, and records an empty value only when the option
                // is the final argument (CLOptions.swift).
                let value = iter
                    .next()
                    .map(|v| v.as_ref().to_string())
                    .unwrap_or_default();
                parsed.values.entry(canonical.long).or_default().push(value);
            }
        }
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_long_and_short_forms() {
        let p = parse(["--title", "Hi", "-m", "There", "--ontop"]);
        assert_eq!(p.value("title"), Some("Hi"));
        assert_eq!(p.value("message"), Some("There"));
        assert!(p.present("ontop"));
    }

    #[test]
    fn deprecated_alias_resolves_to_canonical() {
        let p = parse(["--alignment", "center"]);
        assert_eq!(p.value("messagealignment"), Some("center"));
        assert!(!p.present("alignment"));
    }

    #[test]
    fn unknown_option_is_ignored_like_upstream() {
        // verified live: swiftDialog 3.0.1 shows the dialog and exits
        // normally with --frobnicate present
        let p = parse(["--frobnicate", "--title", "Test"]);
        assert_eq!(p.ignored, ["--frobnicate"]);
        assert_eq!(p.value("title"), Some("Test"));
    }

    #[test]
    fn bare_argument_is_ignored() {
        let p = parse(["hello", "--title", "Hi"]);
        assert_eq!(p.ignored, ["hello"]);
        assert_eq!(p.value("title"), Some("Hi"));
    }

    #[test]
    fn repeatable_options_accumulate_in_order() {
        let p = parse(["--textfield", "Name", "--textfield", "Email,required"]);
        assert_eq!(p.values("textfield"), ["Name", "Email,required"]);
    }

    #[test]
    fn last_value_wins_for_non_repeatable() {
        let p = parse(["--title", "One", "--title", "Two"]);
        assert_eq!(p.value("title"), Some("Two"));
    }

    #[test]
    fn trailing_value_option_records_empty_value() {
        // matches CLOptions.swift: option as final argument -> empty string
        let p = parse(["--message", "hi", "--title"]);
        assert_eq!(p.value("title"), Some(""));
        assert!(p.present("title"));
    }

    #[test]
    fn value_option_consumes_next_argument_unconditionally() {
        // upstream takes argv[i+1] as the value even if it looks like an
        // option — compat over cleverness
        let p = parse(["--title", "--ontop"]);
        assert_eq!(p.value("title"), Some("--ontop"));
        assert!(!p.present("ontop"));
    }

    #[test]
    fn defaults_come_from_the_table() {
        let p = parse(["--message", "hi"]);
        assert_eq!(p.value_or_default("button1text"), Some("OK"));
        assert_eq!(p.value_or_default("width"), Some("820"));
        assert_eq!(p.value_or_default("title"), Some("default-title"));
    }

    #[test]
    fn flag_followed_by_option_does_not_swallow_it() {
        let p = parse(["--ontop", "--title", "Hi"]);
        assert!(p.present("ontop"));
        assert_eq!(p.value("title"), Some("Hi"));
    }
}
