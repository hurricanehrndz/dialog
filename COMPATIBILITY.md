# Compatibility with swiftDialog

Target contract: **swiftDialog v3.0.1** (`.ext/swiftDialog`). This document
records intentional deviations and known gaps, discovered by running our
binary side by side with the real one on the test rig.

## Verified-equivalent behavior

- Exit codes: button1=0, button2=2, info=3, timer=4, `quit:`=5, quitkey=10,
  SIGTERM=40, file-not-found=202 (live-verified for 0/2/4/5).
- **Plain `key : value` output on button1: byte-for-byte identical** to
  swiftDialog 3.0.1 (live-verified by driving a real Return press) — field
  order (textfields → legacy SelectedOption/SelectedIndex → per-name select
  → checkboxes), quoting quirks (textfield line unquoted, others quoted),
  and the bare `SelectedIndex` number all match exactly.
- JSON output content on button1: same keys/values as upstream
  (`SelectedOption`/`SelectedIndex`, per-name `{selectedValue,
  selectedIndex}`, textfields `{name: value}`, checkboxes `{name: bool}`) —
  live-verified. See D-2 for the cosmetic formatting difference.
- Command-file live updates: title/message(+append)/icon/progress/
  progresstext/button text+enable/listitem statuses/list replace/quit
  (verified live on macOS and Windows).
- Unknown CLI options are ignored (dialog still shows), matching upstream.

### D-2: JSON pretty-print formatting (cosmetic)
swiftDialog emits SwiftyJSON's format — `"key" : value` (space before the
colon) with non-deterministic key order. We emit standard `serde_json`
pretty output — `"key": value` (space after only) with sorted keys. The
JSON is **semantically identical**; `jq`, PowerShell `ConvertFrom-Json`,
Python `json`, etc. parse both the same. We do not match the byte format
because upstream's key order is non-deterministic (so unmatchable anyway)
and standard JSON spacing is more portable. Scripts that parse JSON are
unaffected; only a naive raw-string comparison of the JSON blob would
differ. The **plain** output (above), which scripts grep far more often, is
an exact match.

## Intentional deviations

### D-1: JSON + selects on the timer / `--alwaysreturninput` exit path
**Upstream:** when the dialog exits via the timer, swiftDialog calls
`quitDialog(exitCode:4)` **without** passing `observedObject`
(`TimerView.swift:140`). The output routine then reads
`observedObject?.args.jsonOutPut.present ?? false` and
`observedObject?.args.dropdownValues.present ?? false`
(`System.swift:301,372,418`), both of which are `nil` → false. Result on the
timer path: `--json` is ignored (plain `key : value` text) and `select`
values are omitted, even though textfields and checkboxes are emitted.

**Ours:** we emit consistent output on every exit path — `--json` is honored
and selects are included on the timer/`--alwaysreturninput` path as well as
on button1.

**Rationale:** the upstream behavior is an artifact of a nil object, not a
documented contract; a script relying on it would be relying on a bug. We
match the **button1** path exactly (the intended path) and are a strict
superset elsewhere. Scripts that parse button1 output are unaffected.

### D-3: Icons rendered via bundled Fluent glyphs, not native SF Symbols
**Upstream:** `--icon SF=<name>` renders the actual Apple SF Symbol natively
on macOS; builtin `warning`/`caution`/`info` use Apple's symbol imagery.

**Ours:** one cross-platform render path — every `SF=<name>` maps through a
curated table to a glyph in the bundled, MIT-licensed **Fluent System Icons**
font (`ui/fonts/`), drawn identically on macOS and Windows. We don't render
true SF Symbols because doing so needs a native, non-redistributable shim on
macOS (design Q3); Apple's font isn't licensed for redistribution or for
Windows. `.fill` selects the filled variant; `colour=`/`palette=` modifiers
apply as a CSS tint. Builtin `warning`/`caution`/`info` are the Fluent
warning/info glyphs with approximate tints.

**Rationale / impact:** glyph *shape* differs from SF (a Fluent gear vs an
Apple gear) but intent is preserved and scripts are unchanged. Future work:
a native SF-Symbol pass on macOS for fidelity, and per-platform divergence.

**Currently deferred (degrade → default icon + stderr warning):**
- Native `.app`/`.exe`/`.lnk` application-icon extraction (needs platform FFI).
- SF override pack (user-supplied `name.svg/png` directory) for unmapped names.
- Curated SF table covers ~65 common community symbols; unmapped names show a
  placeholder glyph + warning.
- `.icns` files (no webview can render them) → default icon + warning.

`--icon https://…` is downloaded at launch (5 s timeout, 20 MB cap) and
degrades to the default icon on failure. Note: `--checksum <value>` is **not**
icon verification — it's swiftDialog's SHA256 utility (prints the hash of
`<value>` and exits 0, for use with `--authkey`), implemented as such.

### D-4: `--windowbuttons` is a flag, not `<min,max,close>`
Our `window-behavior` spec described `--windowbuttons <min,max,close>` granularity,
but upstream declares it `isbool: true` (`CommandLineArguments.swift:150`) — a plain
flag. We match upstream: `--windowbuttons` shows the native title bar (with its close
control); there is no per-button selection. Closing the window (its close button, the
OS close, or Cmd+W) exits **15** (live-verified). `--fullscreen` covers the display
with a dim backdrop and centers the dialog as a card (swiftDialog renders a full-screen
backdrop); not pixel-identical, but the same intent.

### D-5: `--blurscreen` is a best-effort overlay; `--showonallscreens` dropped
`--blurscreen` covers the primary display with a transparent, always-on-top
overlay that dims the desktop and applies a CSS `backdrop-filter` blur. On
macOS the blur takes effect and the overlay is raised above the menu bar and
dock (NSWindow level via a small `objc2` call — `main.rs`), so it covers all
screen chrome. On Windows it degrades to dim, and the **taskbar remains
visible** (accepted — a topmost window doesn't cover it; borderless-fullscreen
would but isn't worth the divergence). A warning is logged either way and the
dialog stays fully interactive (live-verified on both platforms). This avoids a
native compositor/vibrancy shim (cf. the SF-Symbols decision). True
multi-display overlays (`--showonallscreens`) are **dropped** — the overlay is
the primary display only; the flag otherwise warns and no-ops.

## Known gaps (tracked in tasks.md)

- Icons: native app/exe icon extraction and SF override packs deferred
  (group 5, see D-3) — those sources fall back to the default icon.
- `--blurscreen`, `--fullscreen` window effects (group 10).
- Regex validation uses a small anchored-pattern matcher; patterns it can't
  model (alternation, groups) pass rather than block the user.
- macOS-only Tier 3 options (notification, dock icon, loginwindow) no-op on
  all platforms for now.
