//! The swiftDialog option table — the single source of truth for the CLI
//! surface. Mirrors `CommandLineArguments.swift` from swiftDialog v3.0.1
//! (132 options). Parsing accepts everything in this table; whether an
//! option is *acted on* is decided by the implemented-set at runtime
//! (degrade-don't-break, design D3).

/// How an option consumes arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Boolean flag; takes no value.
    Flag,
    /// Takes one value argument.
    Value,
}

/// Roadmap tier from the proposal. Planning metadata, not runtime gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// v1 scope: core content, icons, buttons, inputs, list/progress, IPC,
    /// basic window behavior.
    T1,
    /// v1.x scope: banners, media, help sheet, blur, advanced window.
    T2,
    /// macOS-only system integration; accepted and no-op'd elsewhere.
    T3,
    /// Stubbed or dropped (builder, easter eggs, jamfHelper mode, Inspect).
    T4,
}

/// Platforms an option can ever be implemented on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platforms {
    All,
    MacOnly,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionDef {
    pub long: &'static str,
    pub short: Option<&'static str>,
    pub kind: Kind,
    /// Compat default as a string, where swiftDialog defines one.
    /// `default-title`/`default-message` are upstream localization sentinels
    /// (VERIFY-LIVE against the real binary in the compat harness).
    pub default: Option<&'static str>,
    pub tier: Tier,
    pub platforms: Platforms,
    /// Hidden from help, still parsed (matches upstream `hidden:`).
    pub hidden: bool,
    /// May appear multiple times on one command line.
    pub repeatable: bool,
    /// Deprecated alias: parse, then treat as this canonical option.
    pub alias_of: Option<&'static str>,
}

const fn opt(long: &'static str, kind: Kind, tier: Tier) -> OptionDef {
    OptionDef {
        long,
        short: None,
        kind,
        default: None,
        tier,
        platforms: Platforms::All,
        hidden: false,
        repeatable: false,
        alias_of: None,
    }
}

const fn short(mut o: OptionDef, s: &'static str) -> OptionDef {
    o.short = Some(s);
    o
}
const fn default(mut o: OptionDef, d: &'static str) -> OptionDef {
    o.default = Some(d);
    o
}
const fn mac_only(mut o: OptionDef) -> OptionDef {
    o.platforms = Platforms::MacOnly;
    o
}
const fn hidden(mut o: OptionDef) -> OptionDef {
    o.hidden = true;
    o
}
const fn repeatable(mut o: OptionDef) -> OptionDef {
    o.repeatable = true;
    o
}
const fn alias_of(mut o: OptionDef, canonical: &'static str) -> OptionDef {
    o.alias_of = Some(canonical);
    o
}

use Kind::{Flag, Value};
use Tier::{T1, T2, T3, T4};

/// All 132 options, in `CommandLineArguments.swift` declaration order.
pub const OPTIONS: &[OptionDef] = &[
    // ── Core content ────────────────────────────────────────────────
    short(default(opt("title", Value, T1), "default-title"), "t"),
    opt("subtitle", Value, T1),
    short(default(opt("message", Value, T1), "default-message"), "m"),
    opt("style", Value, T1),
    default(opt("messagealignment", Value, T1), "left"),
    default(opt("helpalignment", Value, T2), "left"),
    alias_of(
        default(opt("alignment", Value, T1), "left"),
        "messagealignment",
    ),
    opt("messageposition", Value, T1),
    // ── Help sheet ──────────────────────────────────────────────────
    opt("helpmessage", Value, T2),
    opt("helpimage", Value, T2),
    default(opt("helpsheetbuttontext", Value, T2), "OK"),
    // ── Icon & branding ─────────────────────────────────────────────
    short(default(opt("icon", Value, T1), "default"), "i"),
    default(opt("iconsize", Value, T1), "150"),
    default(opt("iconalpha", Value, T1), "1.0"),
    default(opt("iconalttext", Value, T1), "Dialog Icon"),
    short(opt("overlayicon", Value, T1), "y"),
    short(opt("bannerimage", Value, T2), "n"),
    default(opt("bannertitle", Value, T2), "default-title"),
    default(opt("bannertext", Value, T2), "default-title"),
    opt("bannerheight", Value, T2),
    // ── Buttons ─────────────────────────────────────────────────────
    default(opt("button1text", Value, T1), "OK"),
    opt("button1action", Value, T1),
    hidden(opt("button1shellaction", Value, T4)),
    opt("button1symbol", Value, T1),
    default(opt("button2text", Value, T1), "Cancel"),
    opt("button2action", Value, T1),
    opt("button2symbol", Value, T1),
    default(opt("infobuttontext", Value, T1), "More Information"),
    opt("infobuttonaction", Value, T1),
    opt("infobuttonsymbol", Value, T1),
    opt("buttonstyle", Value, T1),
    default(opt("buttonsize", Value, T1), "regular"),
    opt("buttontextsize", Value, T1),
    // ── Input: selects ──────────────────────────────────────────────
    repeatable(opt("selecttitle", Value, T1)),
    repeatable(opt("selectvalues", Value, T1)),
    repeatable(opt("selectdefault", Value, T1)),
    repeatable(opt("selectstyle", Value, T1)),
    // ── Fonts ───────────────────────────────────────────────────────
    opt("titlefont", Value, T1),
    opt("messagefont", Value, T1),
    // ── Input: textfields & checkboxes ──────────────────────────────
    repeatable(opt("textfield", Value, T1)),
    opt("textfieldlivevalidation", Flag, T1),
    repeatable(opt("checkbox", Value, T1)),
    opt("checkboxstyle", Value, T1),
    // ── Timer & progress ────────────────────────────────────────────
    default(opt("timer", Value, T1), "10"),
    opt("progress", Value, T1),
    default(opt("progresstext", Value, T1), " "),
    default(opt("progresstextalignment", Value, T1), " "),
    // ── Media ───────────────────────────────────────────────────────
    short(opt("image", Value, T2), "g"),
    opt("imagecaption", Value, T2),
    // ── Window geometry & background ────────────────────────────────
    default(opt("width", Value, T1), "820"),
    default(opt("height", Value, T1), "380"),
    short(opt("background", Value, T2), "bg"),
    short(opt("bgalpha", Value, T2), "ba"),
    short(opt("bgposition", Value, T2), "bp"),
    short(opt("bgfill", Value, T2), "bf"),
    short(opt("bgscale", Value, T2), "bs"),
    opt("position", Value, T1),
    default(opt("positionoffset", Value, T1), "16"),
    // ── Media (video) ───────────────────────────────────────────────
    opt("video", Value, T2),
    opt("videocaption", Value, T2),
    // ── Diagnostics & IPC ───────────────────────────────────────────
    opt("debug", Value, T1),
    opt("jsonfile", Value, T1),
    opt("jsonstring", Value, T1),
    opt("commandfile", Value, T1),
    // ── List items ──────────────────────────────────────────────────
    repeatable(opt("listitem", Value, T1)),
    opt("liststyle", Value, T1),
    opt("enablelistselect", Flag, T1),
    // ── Info area ───────────────────────────────────────────────────
    opt("infotext", Value, T1),
    opt("infobox", Value, T1),
    // ── Lifecycle & security ────────────────────────────────────────
    default(opt("quitkey", Value, T1), "q"),
    opt("webcontent", Value, T2),
    short(opt("key", Value, T1), "k"),
    opt("checksum", Value, T1),
    opt("displaylog", Value, T2),
    default(opt("loghistory", Value, T2), "100"),
    opt("vieworder", Value, T2),
    opt("appearance", Value, T1),
    mac_only(opt("seticon", Value, T3)),
    short(opt("identifier", Value, T2), "id"),
    hidden(default(opt("pid", Value, T2), "0")),
    mac_only(opt("sound", Value, T3)),
    mac_only(opt("dockicon", Value, T3)),
    mac_only(opt("dockiconbadge", Value, T3)),
    opt("onadvance", Value, T2),
    // ── Boolean flags ───────────────────────────────────────────────
    opt("button1disabled", Flag, T1),
    opt("button2disabled", Flag, T1),
    short(opt("button2", Flag, T1), "2"),
    short(opt("infobutton", Flag, T1), "3"),
    short(opt("version", Flag, T1), "v"),
    short(opt("hideicon", Flag, T1), "h"),
    opt("centreicon", Flag, T1),
    hidden(alias_of(opt("centericon", Flag, T1), "centreicon")),
    opt("help", Flag, T1),
    opt("demo", Flag, T4),
    hidden(short(opt("coffee", Flag, T4), "☕️")),
    short(opt("licence", Flag, T4), "l"),
    opt("warningicon", Flag, T1),
    opt("infoicon", Flag, T1),
    opt("cautionicon", Flag, T1),
    opt("hidetimerbar", Flag, T1),
    opt("hidetimer", Flag, T1),
    opt("autoplay", Flag, T2),
    opt("blurscreen", Flag, T2),
    mac_only(opt("notification", Flag, T3)),
    short(opt("verbose", Flag, T1), "vvv"),
    mac_only(opt("showdockicon", Flag, T3)),
    opt("builder", Flag, T4),
    short(opt("moveable", Flag, T1), "o"),
    short(opt("ontop", Flag, T1), "p"),
    short(opt("small", Flag, T1), "s"),
    short(opt("big", Flag, T1), "b"),
    short(opt("fullscreen", Flag, T2), "f"),
    opt("quitoninfo", Flag, T1),
    opt("listfonts", Flag, T4),
    short(opt("json", Flag, T1), "j"),
    mac_only(short(opt("ignorednd", Flag, T3), "d")),
    short(opt("jh", Flag, T4), "jh"),
    opt("mini", Flag, T1),
    opt("eula", Flag, T4),
    opt("presentation", Flag, T2),
    opt("windowbuttons", Flag, T1),
    opt("resizable", Flag, T1),
    opt("showonallscreens", Flag, T2),
    mac_only(opt("enablenotificationsounds", Flag, T3)),
    mac_only(opt("loginwindow", Flag, T3)),
    opt("hidedefaultkeyboardaction", Flag, T1),
    opt("alwaysreturninput", Flag, T1),
    mac_only(opt("remove", Flag, T3)),
    opt("showsoundcontrols", Flag, T2),
    opt("hideotherapps", Flag, T2),
    // ── Inspect framework (out of scope) ────────────────────────────
    opt("inspect-mode", Flag, T4),
    opt("inspect-config", Value, T4),
];

/// Look up an option by its long name.
pub fn by_long(name: &str) -> Option<&'static OptionDef> {
    OPTIONS.iter().find(|o| o.long == name)
}

/// Look up an option by its short alias.
pub fn by_short(name: &str) -> Option<&'static OptionDef> {
    OPTIONS.iter().find(|o| o.short == Some(name))
}

/// Resolve an option to its canonical definition, following deprecated
/// aliases (e.g. `--alignment` resolves to `messagealignment`).
pub fn canonical(def: &'static OptionDef) -> &'static OptionDef {
    match def.alias_of {
        Some(name) => by_long(name).expect("alias target must exist in OPTIONS"),
        None => def,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The table must mirror swiftDialog v3.0.1's 132-option surface.
    #[test]
    fn table_covers_full_upstream_surface() {
        assert_eq!(OPTIONS.len(), 132);
    }

    #[test]
    fn long_names_are_unique() {
        let mut seen = HashSet::new();
        for o in OPTIONS {
            assert!(seen.insert(o.long), "duplicate long name: {}", o.long);
        }
    }

    #[test]
    fn short_names_are_unique() {
        let mut seen = HashSet::new();
        for o in OPTIONS.iter().filter_map(|o| o.short) {
            assert!(seen.insert(o), "duplicate short name: {o}");
        }
    }

    #[test]
    fn aliases_resolve_to_existing_canonical_options() {
        for o in OPTIONS {
            if let Some(target) = o.alias_of {
                let c = by_long(target).expect("alias target exists");
                assert!(c.alias_of.is_none(), "alias chains are not allowed");
                // An alias must agree with its canonical option on shape.
                assert_eq!(c.kind, o.kind, "alias {} kind mismatch", o.long);
            }
        }
    }

    /// Exact-value spot checks against `CommandLineArguments.swift` /
    /// `AppVariables.swift` (file:line refs in the specs).
    #[test]
    fn compat_defaults_match_upstream() {
        assert_eq!(by_long("width").unwrap().default, Some("820"));
        assert_eq!(by_long("height").unwrap().default, Some("380"));
        assert_eq!(by_long("timer").unwrap().default, Some("10"));
        assert_eq!(by_long("button1text").unwrap().default, Some("OK"));
        assert_eq!(by_long("button2text").unwrap().default, Some("Cancel"));
        assert_eq!(by_long("quitkey").unwrap().default, Some("q"));
        assert_eq!(by_long("iconsize").unwrap().default, Some("150"));
        assert_eq!(
            by_long("infobuttontext").unwrap().default,
            Some("More Information")
        );
    }

    #[test]
    fn short_aliases_match_upstream() {
        for (s, l) in [
            ("t", "title"),
            ("m", "message"),
            ("i", "icon"),
            ("2", "button2"),
            ("j", "json"),
            ("v", "version"),
            ("bg", "background"),
        ] {
            assert_eq!(by_short(s).unwrap().long, l);
        }
    }
}
