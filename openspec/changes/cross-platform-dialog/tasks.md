## 1. Project scaffolding

- [ ] 1.1 Initialize Tauri 2.x workspace (Rust core crate + minimal TS frontend, no framework decision yet beyond Q1 in design.md) building and opening a blank window on macOS and Windows
- [x] 1.2 Set up CI for both platforms (build + test), including the Windows `AttachConsole` stdout integration test harness skeleton
- [x] 1.3 Resolve design Q1 (frontend layer) and Q2 (Windows default command-file path + ACL); record decisions in design.md

## 2. Option table and CLI parsing (cli-compatibility)

- [x] 2.1 Transcribe all 132 options from `CommandLineArguments.swift` into the declarative option table (name, short alias, type, default, tier, platform availability)
- [x] 2.2 Implement the argument parser over the table: long/short flags, bools vs values, deprecated aliases, unknown-option error path
- [x] 2.3 Implement degrade-don't-break: warn-and-noop for known-but-unimplemented or platform-unavailable options
- [x] 2.4 Implement `--jsonfile`/`--jsonstring` input mapping onto the same option table, including array forms (textfield, checkbox, selectitems, listitem)
- [x] 2.5 Implement exit-code constants and the quit path (codes 0/2/3/4/5/10/15/20/30/40/201/202/203/255), SIGTERM handler included — constants done; process quit path + SIGTERM wiring lands with the app shell (task 3.2)
- [x] 2.6 Implement output writer: plain `key : value` lines and `--json` object output with swiftDialog's exact key naming (incl. legacy SelectedOption/SelectedIndex)
- [ ] 2.7 Windows: AttachConsole on startup; verify stdout capture from PowerShell in CI
- [x] 2.8 `--version`, `--help`, `--verbose` output

## 3. Dialog state model and renderer bridge

- [ ] 3.1 Define the `DialogState` model in Rust (serde-serializable) covering Tier 1 surface; full-state push + patch events to the webview
- [ ] 3.2 Define semantic UI event channel from webview to Rust (button clicks, input changes, list selection) wired to exit/output logic
- [ ] 3.3 Frontend shell: render title/subtitle/markdown message/icon area/button bar from DialogState; platform theme switch (macOS vs Fluent tokens)

## 4. Core content (dialog-core)

- [ ] 4.1 Markdown rendering in message body (CommonMark; links open default browser via Rust)
- [ ] 4.2 Title/message font options (`name=,size=,weight=,colour=`), alignment and vertical position options
- [ ] 4.3 Default 820×380 window, `--width`/`--height`, size presets `--small`/`--big`/`--mini` and `--style` layouts
- [ ] 4.4 Light/dark appearance: follow OS, `--appearance` override
- [ ] 4.5 Background image with alpha/position/fill options

## 5. Icons and branding (icon-branding)

- [ ] 5.1 Icon resolution module: file path, URL download (+ `--checksum` verification), builtin keywords, `--iconsize`, `none` reflow
- [ ] 5.2 macOS `SF=name` rendering and `.app` icon extraction
- [ ] 5.3 Windows: SF→Segoe Fluent Icons mapping table (curate top ~200 community-used symbols), `.exe`/`.lnk` icon extraction, override-pack lookup, placeholder fallback
- [ ] 5.4 Overlay icon, built-in warning/info/caution icons
- [ ] 5.5 Banner image with `--bannertitle`/`--bannerheight`

## 6. Buttons and lifecycle (buttons-actions)

- [ ] 6.1 Button 1/2/info rendering, default labels (OK/Cancel), enable/disable, actions (open URL), exit-code wiring
- [ ] 6.2 Keyboard handling: Return = button1, Escape = button2, quitkey (Cmd/Ctrl+char) → exit 10
- [ ] 6.3 Timer with countdown bar, default 10 s, `--hidetimerbar`, exit 4
- [ ] 6.4 Button styles (`stack`/`center`), sizes, button symbols via icon pipeline

## 7. User input (user-input)

- [ ] 7.1 Textfields: sub-option parser (required/secure/prompt/value/regex/regexerror/name), rendering, masking
- [ ] 7.2 Submit-time validation: required highlight + error sheet, regex check, don't-quit behavior
- [ ] 7.3 Checkboxes and switch style; disabled state
- [ ] 7.4 Dropdown selects and radio style; required validation; multiple groups
- [ ] 7.5 Output integration for all input types incl. `--alwaysreturninput`

## 8. List and progress (list-progress)

- [ ] 8.1 List rendering: rows with title/subtitle/icon/status/statustext, scrolling
- [ ] 8.2 Status values wait/progress/success/fail/error/pending/none with platform glyphs and spinner animation
- [ ] 8.3 Overall progress bar: determinate/indeterminate, progresstext
- [ ] 8.4 `--enablelistselect` selectable rows + output contribution

## 9. Command file IPC (command-file-ipc)

- [ ] 9.1 File watcher: notify + polling fallback, seek-offset tailing, truncation reset, 0666/ACL creation, default paths per platform
- [ ] 9.2 Verb parser and dispatch onto DialogState (content verbs: title/subtitle/message+append/icon/image/progress/progresstext/infotext/infobox)
- [ ] 9.3 List verbs: `listitem:` by title/index, add/delete, per-row progress, `list:` replace/clear
- [ ] 9.4 Control verbs: button text/enable/disable, `quit:` (exit 5), `activate:`, width/height/position
- [ ] 9.5 Malformed-line tolerance and debug logging

## 10. Window behavior (window-behavior)

- [ ] 10.1 Positioning (9 anchors + offset), ontop, moveable, resizable, windowbuttons (+ close → exit 15)
- [ ] 10.2 Fullscreen backdrop mode
- [ ] 10.3 Blur/dim overlay: macOS blur, Windows best-effort (dim fallback + warning), `--showonallscreens` multi-display overlays

## 11. Compatibility verification

- [ ] 11.1 Build the compat harness: table-driven cases of invocation → expected exit code/stdout, and command-file scripts → expected state
- [ ] 11.2 Record fixture cases from real community scripts (Setup-Your-Mac, Baseline patterns) and swiftDialog docs examples
- [ ] 11.3 macOS CI job running real swiftDialog side-by-side to detect contract drift
- [ ] 11.4 Write COMPATIBILITY.md documenting tier status and known deviations per option
- [ ] 11.5 Cold-start benchmark (<500 ms to visible window target) on both platforms in CI
