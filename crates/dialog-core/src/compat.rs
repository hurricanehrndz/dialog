//! Degrade-don't-break: every option in the table parses, but options that
//! aren't implemented yet (or can never work on this platform) warn on
//! stderr and no-op. A script with `--blurscreen` must still show its
//! dialog (cli-compatibility spec).

use crate::options::{self, Platforms};
use crate::parser::ParsedArgs;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

impl Platform {
    pub const fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Platform::MacOs
        }
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Platform::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradeReason {
    /// Feature hasn't been built yet (tier not reached).
    NotImplemented,
    /// Option can never apply on this platform (e.g. `--loginwindow` on
    /// Windows).
    PlatformUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DegradeWarning {
    pub option: &'static str,
    pub reason: DegradeReason,
}

impl std::fmt::Display for DegradeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self.reason {
            DegradeReason::NotImplemented => "not yet implemented",
            DegradeReason::PlatformUnavailable => "not available on this platform",
        };
        write!(f, "WARNING: --{} {} — ignoring", self.option, reason)
    }
}

/// Compute warnings for present options that will be ignored. The caller
/// owns `implemented` (the registry of options actually wired up) and is
/// responsible for printing the warnings to stderr.
pub fn degrade_warnings(
    args: &ParsedArgs,
    implemented: &HashSet<&str>,
    platform: Platform,
) -> Vec<DegradeWarning> {
    let mut warnings = Vec::new();
    for name in args.present_options() {
        let def = options::by_long(name).expect("parsed options come from the table");
        if def.platforms == Platforms::MacOnly && platform != Platform::MacOs {
            warnings.push(DegradeWarning {
                option: name,
                reason: DegradeReason::PlatformUnavailable,
            });
        } else if !implemented.contains(name) {
            warnings.push(DegradeWarning {
                option: name,
                reason: DegradeReason::NotImplemented,
            });
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn implemented(names: &[&'static str]) -> HashSet<&'static str> {
        names.iter().copied().collect()
    }

    #[test]
    fn unimplemented_option_warns_but_parses() {
        let args = parse(["--title", "Hi", "--blurscreen"]);
        let warnings = degrade_warnings(&args, &implemented(&["title"]), Platform::Windows);
        assert_eq!(
            warnings,
            vec![DegradeWarning {
                option: "blurscreen",
                reason: DegradeReason::NotImplemented,
            }]
        );
    }

    #[test]
    fn mac_only_option_is_platform_unavailable_on_windows() {
        let args = parse(["--notification"]);
        let warnings = degrade_warnings(&args, &implemented(&["notification"]), Platform::Windows);
        assert_eq!(warnings[0].reason, DegradeReason::PlatformUnavailable);
    }

    #[test]
    fn mac_only_option_on_macos_falls_through_to_implemented_check() {
        let args = parse(["--notification"]);
        let warnings = degrade_warnings(&args, &implemented(&["notification"]), Platform::MacOs);
        assert!(warnings.is_empty());
    }

    #[test]
    fn implemented_options_do_not_warn() {
        let args = parse(["--title", "Hi", "--ontop"]);
        let warnings = degrade_warnings(&args, &implemented(&["title", "ontop"]), Platform::MacOs);
        assert!(warnings.is_empty());
    }

    #[test]
    fn warning_message_format() {
        let w = DegradeWarning {
            option: "blurscreen",
            reason: DegradeReason::NotImplemented,
        };
        assert_eq!(
            w.to_string(),
            "WARNING: --blurscreen not yet implemented — ignoring"
        );
    }
}
