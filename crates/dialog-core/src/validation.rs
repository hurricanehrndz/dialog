//! Submit-time validation for user input (user-input spec). On button1,
//! the shell collects current field values and runs these checks; any
//! failure blocks the quit and surfaces an error sheet, matching
//! swiftDialog (System.swift required/regex handling).

use crate::state::DialogState;

/// One unmet requirement, rendered as a line in the error sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

/// Current values submitted from the renderer, keyed by element name.
#[derive(Debug, Clone, Default)]
pub struct Submission {
    pub text_fields: Vec<(String, String)>,
    pub selects: Vec<(String, String)>,
}

impl Submission {
    fn text(&self, name: &str) -> Option<&str> {
        self.text_fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    fn select(&self, name: &str) -> Option<&str> {
        self.selects
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Validate the submission against the dialog's declared requirements.
/// Empty result == may quit. Uses a minimal regex engine (anchored ^…$
/// patterns with classes/quantifiers) sufficient for the serial/email
/// patterns scripts actually use; complex patterns fall back to "match".
pub fn validate(state: &DialogState, sub: &Submission) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for field in &state.text_fields {
        let value = sub.text(&field.name).unwrap_or("");
        if field.required && value.trim().is_empty() {
            errors.push(ValidationError {
                message: format!("\"{}\" is required", field.title),
            });
            continue;
        }
        if !value.is_empty() {
            if let Some(pattern) = &field.regex {
                if !regex_lite::matches(pattern, value) {
                    let msg = field
                        .regex_error
                        .clone()
                        .unwrap_or_else(|| format!("\"{}\" is invalid", field.title));
                    errors.push(ValidationError { message: msg });
                }
            }
        }
    }

    for select in &state.selects {
        let value = sub.select(&select.name).unwrap_or("");
        if select.required && value.trim().is_empty() {
            errors.push(ValidationError {
                message: format!("\"{}\" is required", select.title),
            });
        }
    }

    errors
}

/// A deliberately small regex matcher covering the anchored character-class
/// patterns endpoint scripts use (`^[A-Z0-9]{10}$`, `^.+@.+\..+$`, etc.).
/// Not a general engine — see VERIFY-LIVE note: complex patterns that this
/// can't model conservatively return `true` (don't block the user).
mod regex_lite {
    pub fn matches(pattern: &str, text: &str) -> bool {
        // Only anchored patterns are validated; unanchored → don't block.
        let Some(inner) = pattern.strip_prefix('^').and_then(|p| p.strip_suffix('$')) else {
            return true;
        };
        let tokens = match tokenize(inner) {
            Some(t) => t,
            None => return true, // unsupported syntax → don't block
        };
        match_tokens(&tokens, &text.chars().collect::<Vec<_>>())
    }

    #[derive(Debug)]
    struct Token {
        class: Class,
        min: usize,
        max: usize, // usize::MAX = unbounded
    }

    #[derive(Debug)]
    enum Class {
        Any,
        Literal(char),
        Set {
            ranges: Vec<(char, char)>,
            negate: bool,
        },
    }

    fn tokenize(pat: &str) -> Option<Vec<Token>> {
        let chars: Vec<char> = pat.chars().collect();
        let mut i = 0;
        let mut out = Vec::new();
        while i < chars.len() {
            let class = match chars[i] {
                '.' => {
                    i += 1;
                    Class::Any
                }
                '[' => {
                    let close = chars[i..].iter().position(|&c| c == ']')? + i;
                    let mut body = &chars[i + 1..close];
                    let negate = body.first() == Some(&'^');
                    if negate {
                        body = &body[1..];
                    }
                    let mut ranges = Vec::new();
                    let mut j = 0;
                    while j < body.len() {
                        if j + 2 < body.len() && body[j + 1] == '-' {
                            ranges.push((body[j], body[j + 2]));
                            j += 3;
                        } else {
                            ranges.push((body[j], body[j]));
                            j += 1;
                        }
                    }
                    i = close + 1;
                    Class::Set { ranges, negate }
                }
                '\\' => {
                    let c = *chars.get(i + 1)?;
                    i += 2;
                    Class::Literal(c)
                }
                // Unsupported metacharacters → bail out (don't block).
                '(' | ')' | '|' | '?' => return None,
                c => {
                    i += 1;
                    Class::Literal(c)
                }
            };
            let (min, max) = match chars.get(i) {
                Some('+') => {
                    i += 1;
                    (1, usize::MAX)
                }
                Some('*') => {
                    i += 1;
                    (0, usize::MAX)
                }
                Some('{') => {
                    let close = chars[i..].iter().position(|&c| c == '}')? + i;
                    let spec: String = chars[i + 1..close].iter().collect();
                    i = close + 1;
                    match spec.split_once(',') {
                        Some((a, "")) => (a.parse().ok()?, usize::MAX),
                        Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
                        None => {
                            let n = spec.parse().ok()?;
                            (n, n)
                        }
                    }
                }
                _ => (1, 1),
            };
            out.push(Token { class, min, max });
        }
        Some(out)
    }

    fn class_matches(class: &Class, c: char) -> bool {
        match class {
            Class::Any => true,
            Class::Literal(l) => *l == c,
            Class::Set { ranges, negate } => {
                let hit = ranges.iter().any(|(lo, hi)| c >= *lo && c <= *hi);
                hit != *negate
            }
        }
    }

    /// Greedy backtracking match of the full token list against all chars.
    fn match_tokens(tokens: &[Token], text: &[char]) -> bool {
        fn rec(tokens: &[Token], ti: usize, text: &[char], pos: usize) -> bool {
            let Some(tok) = tokens.get(ti) else {
                return pos == text.len();
            };
            // Count how many consecutive chars match this class.
            let mut avail = 0;
            while pos + avail < text.len()
                && avail < tok.max
                && class_matches(&tok.class, text[pos + avail])
            {
                avail += 1;
            }
            if avail < tok.min {
                return false;
            }
            // Greedy: try longest first, backtrack down to min.
            let mut take = avail;
            loop {
                if rec(tokens, ti + 1, text, pos + take) {
                    return true;
                }
                if take == tok.min {
                    return false;
                }
                take -= 1;
            }
        }
        rec(tokens, 0, text, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::parser::parse;

    fn state(args: &[&str]) -> DialogState {
        DialogState::from_config(&Config::load(parse(args.iter().copied())).unwrap())
    }

    fn sub(fields: &[(&str, &str)]) -> Submission {
        Submission {
            text_fields: fields
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
            selects: Vec::new(),
        }
    }

    #[test]
    fn required_field_blocks_when_empty() {
        let s = state(&["--textfield", "Email,required"]);
        let errs = validate(&s, &sub(&[("Email", "")]));
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("Email"));
        assert!(validate(&s, &sub(&[("Email", "a@b.co")])).is_empty());
    }

    #[test]
    fn regex_validation_with_custom_error() {
        let s = state(&[
            "--textfield",
            "Serial,regex=^[A-Z0-9]{10}$,regexerror=Must be 10 characters",
        ]);
        let errs = validate(&s, &sub(&[("Serial", "abc")]));
        assert_eq!(errs[0].message, "Must be 10 characters");
        assert!(validate(&s, &sub(&[("Serial", "ABCD123456")])).is_empty());
    }

    #[test]
    fn email_like_pattern() {
        let s = state(&["--textfield", "Email,regex=^.+@.+\\..+$"]);
        assert!(validate(&s, &sub(&[("Email", "x@y.com")])).is_empty());
        assert_eq!(validate(&s, &sub(&[("Email", "nope")])).len(), 1);
    }

    #[test]
    fn empty_optional_field_skips_regex() {
        let s = state(&["--textfield", "Serial,regex=^[A-Z]+$"]);
        assert!(validate(&s, &sub(&[("Serial", "")])).is_empty());
    }

    #[test]
    fn required_select_blocks() {
        let s = state(&[
            "--selecttitle",
            "Site,required",
            "--selectvalues",
            "NYC,SFO",
        ]);
        let mut submission = Submission::default();
        submission.selects.push(("Site".into(), String::new()));
        assert_eq!(validate(&s, &submission).len(), 1);
        submission.selects[0].1 = "NYC".into();
        assert!(validate(&s, &submission).is_empty());
    }

    #[test]
    fn unsupported_pattern_does_not_block() {
        // alternation isn't modeled → conservative pass (don't trap user)
        let s = state(&["--textfield", "X,regex=^(a|b)$"]);
        assert!(validate(&s, &sub(&[("X", "zzz")])).is_empty());
    }
}
