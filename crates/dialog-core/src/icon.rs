//! Icon source resolution (icon-branding capability, design D6). Classifies
//! `--icon`/`--overlayicon`/`--bannerimage` values and resolves each to a
//! renderable representation the webview can draw:
//!   - image files and `https://` URLs  → base64 `data:` URL (`IconRender::Image`)
//!   - `SF=<name>` and builtin keywords  → a Fluent icon-font glyph (`Glyph`)
//!   - `none`                            → nothing; the icon area reflows
//!
//! One render path on both platforms: rather than render Apple's SF Symbols
//! (which would need a native, non-redistributable shim — design Q3), every
//! `SF=` name maps through a curated table to a glyph in the bundled,
//! MIT-licensed Fluent System Icons font. True SF fidelity on macOS is a
//! future improvement.
//!
//! Deferred (degrade-don't-break → default icon + stderr warning), tracked
//! in COMPATIBILITY.md:
//!   - native `.app`/`.exe`/`.lnk` application-icon extraction (needs FFI);
//!   - SF names outside the curated table → generic placeholder glyph.

use crate::state::DialogState;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Read;

/// What the renderer should draw for an icon slot. Tagged so the webview can
/// switch on `kind` (`none`/`image`/`glyph`/`placeholder`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum IconRender {
    /// Hide the icon area and reflow content (`--icon none`).
    None,
    /// A resolved raster/vector image as a `data:` URL.
    Image { url: String },
    /// A Fluent icon-font glyph: `glyph` is the character, `color` an
    /// optional CSS colour (SF `colour=` modifier or builtin tint).
    Glyph {
        glyph: String,
        color: Option<String>,
    },
    /// Recognised but not renderable here (unmapped SF symbol, deferred
    /// app-icon extraction): the renderer draws its generic placeholder.
    Placeholder,
}

/// Classified `--icon` value, before any I/O.
#[derive(Debug, PartialEq)]
enum IconSource {
    /// `none` / empty → hidden.
    Hidden,
    /// `default` / `warning` / `caution` / `info` builtin keyword.
    Builtin(String),
    /// `SF=<name>[,colour=..,palette=..]`.
    Sf { name: String, color: Option<String> },
    /// `http(s)://…`.
    Url(String),
    /// `.app` / `.exe` / `.lnk` — extraction deferred.
    AppBundle(String),
    /// A local image file path.
    File(String),
}

fn classify(source: &str) -> IconSource {
    let s = source.trim();
    if s.is_empty() {
        return IconSource::Hidden;
    }
    if let Some(spec) = s.strip_prefix("SF=") {
        let parsed = crate::suboptions::parse_spec(spec);
        // colour=/color= win; otherwise the first palette colour.
        let color = parsed
            .get("colour")
            .or_else(|| parsed.get("color"))
            .map(str::to_string)
            .or_else(|| parsed.get("palette").map(str::to_string));
        return IconSource::Sf {
            name: parsed.title.to_lowercase(),
            color,
        };
    }
    match s.to_lowercase().as_str() {
        "none" => return IconSource::Hidden,
        kw @ ("default" | "warning" | "caution" | "info") => {
            return IconSource::Builtin(kw.to_string())
        }
        _ => {}
    }
    if s.starts_with("https://") || s.starts_with("http://") {
        return IconSource::Url(s.to_string());
    }
    let lower = s.to_lowercase();
    if lower.ends_with(".app") || lower.ends_with(".exe") || lower.ends_with(".lnk") {
        return IconSource::AppBundle(s.to_string());
    }
    IconSource::File(s.to_string())
}

/// Resolve one icon source to a render. Performs file/network I/O for
/// file-path and URL sources; everything else is pure.
pub fn resolve(source: &str) -> IconRender {
    match classify(source) {
        IconSource::Hidden => IconRender::None,
        IconSource::Builtin(kw) => builtin(&kw),
        IconSource::Sf { name, color } => match sf_glyph(&name) {
            Some(cp) => glyph(cp, color),
            None => {
                eprintln!("WARNING: SF symbol '{name}' is not in the icon map — using placeholder");
                IconRender::Placeholder
            }
        },
        IconSource::Url(url) => match download(&url) {
            Ok(bytes) => image_data_url(&url, &bytes),
            Err(e) => {
                eprintln!("WARNING: failed to download icon {url}: {e} — using default icon");
                builtin("default")
            }
        },
        IconSource::AppBundle(path) => {
            eprintln!(
                "WARNING: application icon extraction ({path}) is not yet implemented — using default icon"
            );
            builtin("default")
        }
        IconSource::File(path) => match std::fs::read(&path) {
            Ok(bytes) => image_data_url(&path, &bytes),
            Err(e) => {
                eprintln!("WARNING: cannot read icon file {path}: {e} — using default icon");
                builtin("default")
            }
        },
    }
}

/// Resolve a button symbol (`--button1symbol` etc.). swiftDialog button
/// symbols are SF symbol names given bare, so we resolve them through the SF
/// path (pure glyph lookup — no I/O). None for an empty value.
pub fn resolve_symbol(name: &str) -> Option<IconRender> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| resolve(&format!("SF={trimmed}")))
}

/// Resolve the main icon from its current source (startup + `icon:` verb).
pub fn resolve_icon(state: &mut DialogState) {
    state.icon.render = resolve(&state.icon.source);
}

/// Resolve everything that only changes at launch: main icon, overlay badge,
/// banner image. Called once before the window is shown.
pub fn resolve_initial(state: &mut DialogState) {
    resolve_icon(state);
    if let Some(overlay) = state.icon.overlay.clone() {
        state.icon.overlay_render = Some(resolve(&overlay));
    }
    if let Some(banner) = state.banner.as_mut() {
        banner.render = resolve(&banner.source);
    }
    if let Some(bg) = state.background.as_mut() {
        bg.render = resolve(&bg.source);
    }
}

fn glyph(cp: u32, color: Option<String>) -> IconRender {
    IconRender::Glyph {
        glyph: char::from_u32(cp)
            .map(|c| c.to_string())
            .unwrap_or_default(),
        color,
    }
}

/// Builtin keyword imagery, as tinted Fluent glyphs (swiftDialog renders
/// warning/caution/info as coloured symbols; colours approximate upstream).
fn builtin(keyword: &str) -> IconRender {
    match keyword {
        "warning" => glyph(WARNING, Some("#ff9500".into())),
        "caution" => glyph(WARNING, Some("#ffcc00".into())),
        "info" => glyph(INFO, Some("#0a84ff".into())),
        // `default` upstream is the caller's app icon; cross-platform we have
        // no such handle, so show a neutral, theme-tinted info glyph.
        "default" => glyph(INFO, None),
        _ => IconRender::Placeholder,
    }
}

fn image_data_url(path_or_url: &str, bytes: &[u8]) -> IconRender {
    let Some(mime) = image_mime(path_or_url, bytes) else {
        eprintln!(
            "WARNING: unsupported icon format ({path_or_url}); webviews cannot render it — using default icon"
        );
        return builtin("default");
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    IconRender::Image {
        url: format!("data:{mime};base64,{encoded}"),
    }
}

/// Detect a webview-renderable image MIME from magic bytes, falling back to
/// the file extension for text formats (SVG). Returns None for formats no
/// webview can draw (ICNS) so the caller can degrade to the default icon.
fn image_mime(path: &str, bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF8") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.starts_with(b"BM") {
        return Some("image/bmp");
    }
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return Some("image/x-icon");
    }
    if bytes.starts_with(b"icns") {
        return None; // Apple ICNS — not renderable by WKWebView/WebView2.
    }
    let lower = path.to_lowercase();
    if lower.ends_with(".svg") {
        return Some("image/svg+xml");
    }
    // SVG without an extension (e.g. a URL) — sniff the markup.
    let head = &bytes[..bytes.len().min(256)];
    if let Ok(text) = std::str::from_utf8(head) {
        let t = text.trim_start();
        if t.starts_with("<svg") || t.starts_with("<?xml") {
            return Some("image/svg+xml");
        }
    }
    None
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(20_000_000) // 20 MB ceiling — icons are tiny; cap pathological URLs.
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

// Fluent System Icons (Resizable) codepoints. Filled vs regular variant is
// chosen by a trailing `.fill` on the SF name. Named constants reused by the
// builtin keywords above.
const WARNING: u32 = 62745;
const INFO: u32 = 59998;

/// Look up a curated SF symbol name → Fluent codepoint. A trailing `.fill`
/// selects the filled variant. Returns None for names outside the table.
fn sf_glyph(name: &str) -> Option<u32> {
    let (base, filled) = match name.strip_suffix(".fill") {
        Some(b) => (b, true),
        None => (name, false),
    };
    SF_FLUENT
        .iter()
        .find(|(sf, ..)| *sf == base)
        .map(|(_, regular, fill)| if filled { *fill } else { *regular })
}

/// Curated SF-symbol → (regular, filled) Fluent codepoints, covering the
/// symbols common in community swiftDialog scripts. Extend as needed; an
/// unmapped name degrades to a placeholder glyph (icon-branding spec).
#[rustfmt::skip]
static SF_FLUENT: &[(&str, u32, u32)] = &[
    ("gear",                          61397, 61396),
    ("gearshape",                     61397, 61396),
    ("gearshape.2",                   61397, 61396),
    ("checkmark",                     58460, 58459),
    ("checkmark.circle",              58462, 58461),
    ("checkmark.seal",                58462, 58461),
    ("checkmark.shield",              61445, 61444),
    ("shield.checkmark",              61445, 61444),
    ("shield",                        61437, 61436),
    ("lock.shield",                   61437, 61436),
    ("xmark",                         59083, 59082),
    ("xmark.circle",                  59085, 59084),
    ("exclamationmark.triangle",      62745, 62744),
    ("exclamationmark.circle",        59501, 59500),
    ("info.circle",                   59998, 59997),
    ("questionmark.circle",           61139, 61138),
    ("person",                        60799, 60798),
    ("person.2",                      60739, 60738),
    ("person.3",                      60739, 60738),
    ("lock",                          60284, 60283),
    ("wifi",                          62815, 62814),
    ("desktopcomputer",               59007, 59006),
    ("display",                       59007, 59006),
    ("laptopcomputer",                60059, 60058),
    ("globe",                         59767, 59766),
    ("network",                       59767, 59766),
    ("arrow.down.circle",             57563, 57562),
    ("arrow.down",                    57563, 57562),
    ("square.and.arrow.down",         57563, 57562),
    ("arrow.clockwise",               57533, 57532),
    ("arrow.triangle.2.circlepath",   57711, 57710),
    ("trash",                         58983, 58982),
    ("folder",                        59615, 59614),
    ("doc",                           59105, 59104),
    ("doc.text",                      59105, 59104),
    ("bell",                          57387, 57386),
    ("envelope",                      60300, 60299),
    ("house",                         59916, 59915),
    ("magnifyingglass",               61341, 61340),
    ("bolt",                          59575, 59574),
    ("battery.100",                   57821, 57820),
    ("key",                           60029, 60028),
    ("hourglass",                     59938, 59937),
    ("clock",                         58628, 58627),
    ("calendar",                      58156, 58155),
    ("star",                          61725, 61724),
    ("heart",                         59881, 59880),
    ("cloud",                         58654, 58653),
    ("printer",                       61095, 61094),
    ("terminal",                      58698, 58697),
    ("wrench",                        62899, 62898),
    ("wrench.and.screwdriver",        62899, 62898),
    ("camera",                        58304, 58303),
    ("mic",                           60430, 60429),
    ("speaker.wave.2",                61643, 61642),
    ("power",                         61065, 61064),
    ("server.rack",                   61391, 61390),
    ("externaldrive",                 59849, 59848),
    ("internaldrive",                 59849, 59848),
    ("building.2",                    58100, 58099),
    ("iphone",                        60923, 60922),
    ("ipad",                          62007, 62006),
    ("app",                           57461, 57460),
    ("square.grid.2x2",               57461, 57460),
    ("shippingbox",                   58020, 58019),
    ("list.bullet",                   62152, 62151),
    ("square.and.arrow.up",           61279, 61278),
    ("circle",                        58512, 58511),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_and_empty_hide() {
        assert_eq!(resolve("none"), IconRender::None);
        assert_eq!(resolve("  "), IconRender::None);
    }

    #[test]
    fn builtins_are_tinted_glyphs() {
        let IconRender::Glyph { color, .. } = resolve("warning") else {
            panic!("expected glyph");
        };
        assert_eq!(color.as_deref(), Some("#ff9500"));
        assert!(matches!(resolve("info"), IconRender::Glyph { .. }));
        // caution reuses the warning glyph with a different tint.
        let (IconRender::Glyph { glyph: w, .. }, IconRender::Glyph { glyph: c, .. }) =
            (resolve("warning"), resolve("caution"))
        else {
            panic!("expected glyphs");
        };
        assert_eq!(w, c);
    }

    #[test]
    fn sf_maps_to_glyph_and_fill_picks_variant() {
        let regular = sf_glyph("checkmark.circle").unwrap();
        let filled = sf_glyph("checkmark.circle.fill").unwrap();
        assert_ne!(regular, filled);
        assert_eq!(regular, 58462);
        assert_eq!(filled, 58461);
        // gear is an alias for the settings glyph.
        assert_eq!(sf_glyph("gear"), sf_glyph("gearshape"));
    }

    #[test]
    fn sf_spec_parses_colour_modifier() {
        let IconSource::Sf { name, color } = classify("SF=gear,colour=#ff0000,weight=bold") else {
            panic!("expected SF source");
        };
        assert_eq!(name, "gear");
        assert_eq!(color.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn sf_colour_applies_to_glyph() {
        let IconRender::Glyph { color, .. } = resolve("SF=gear,color=blue") else {
            panic!("expected glyph");
        };
        assert_eq!(color.as_deref(), Some("blue"));
    }

    #[test]
    fn unmapped_sf_falls_back_to_placeholder() {
        assert_eq!(resolve("SF=obscure.symbol.name"), IconRender::Placeholder);
    }

    #[test]
    fn app_bundle_paths_are_deferred_to_default() {
        // .app/.exe/.lnk extraction is not implemented → default icon glyph.
        assert!(matches!(
            resolve("/Applications/Safari.app"),
            IconRender::Glyph { .. }
        ));
        assert!(matches!(
            resolve("C:\\Windows\\System32\\notepad.exe"),
            IconRender::Glyph { .. }
        ));
    }

    #[test]
    fn classify_distinguishes_sources() {
        assert!(matches!(classify("https://x/y.png"), IconSource::Url(_)));
        assert!(matches!(classify("/tmp/logo.png"), IconSource::File(_)));
        assert!(matches!(classify("Foo.app"), IconSource::AppBundle(_)));
    }

    #[test]
    fn file_path_becomes_a_png_data_url() {
        // 1x1 transparent PNG.
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let dir = std::env::temp_dir();
        let path = dir.join("dialog_icon_test.png");
        std::fs::write(&path, png).unwrap();
        let render = resolve(path.to_str().unwrap());
        let IconRender::Image { url } = render else {
            panic!("expected image");
        };
        assert!(url.starts_with("data:image/png;base64,"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn missing_file_degrades_to_default() {
        assert!(matches!(
            resolve("/no/such/icon/path.png"),
            IconRender::Glyph { .. }
        ));
    }

    #[test]
    fn svg_detected_by_extension() {
        assert_eq!(
            image_mime("logo.svg", b"<svg></svg>"),
            Some("image/svg+xml")
        );
        assert_eq!(image_mime("x", b"<svg "), Some("image/svg+xml"));
    }

    #[test]
    fn icns_is_rejected() {
        assert_eq!(image_mime("x.icns", b"icns\0\0\0\0"), None);
    }
}
