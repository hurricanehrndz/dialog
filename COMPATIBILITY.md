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

## Known gaps (tracked in tasks.md)

- Icons: `SF=` symbols and app/exe icon extraction not yet implemented
  (group 5) — placeholder glyph shown.
- `--blurscreen`, `--fullscreen` window effects (group 10).
- Regex validation uses a small anchored-pattern matcher; patterns it can't
  model (alternation, groups) pass rather than block the user.
- macOS-only Tier 3 options (notification, dock icon, loginwindow) no-op on
  all platforms for now.
