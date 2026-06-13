# Compatibility with swiftDialog

Target contract: **swiftDialog v3.0.1** (`.ext/swiftDialog`). This document
records intentional deviations and known gaps, discovered by running our
binary side by side with the real one on the test rig.

## Verified-equivalent behavior

- Exit codes: button1=0, button2=2, info=3, timer=4, `quit:`=5, quitkey=10,
  SIGTERM=40, file-not-found=202 (live-verified for 0/2/4/5).
- JSON output structure on button1: `SelectedOption`/`SelectedIndex` (single
  select), per-name `{selectedValue, selectedIndex}`, textfields as
  `{name: value}`, checkboxes as `{name: bool}` — matches `System.swift`.
- Command-file live updates: title/message(+append)/icon/progress/
  progresstext/button text+enable/listitem statuses/list replace/quit.
- Unknown CLI options are ignored (dialog still shows), matching upstream.

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
