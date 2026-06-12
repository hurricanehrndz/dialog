## Why

swiftDialog is the de facto scriptable dialog utility for macOS admins (one binary: flags in, native dialog up, JSON + exit code out), but nothing equivalent exists for Windows — admins assemble PSADT, toast scripts, and WPF hacks. Research confirmed the niche is empty: no cross-platform reimplementation exists. We will build a cross-platform (Windows + macOS) dialog utility that acts as a drop-in replacement for swiftDialog, so existing scripts and workflows (Setup-Your-Mac-style onboarding, deployment progress UIs, user prompts) work unchanged to a reasonable extent on both platforms.

## What Changes

- New application: a single CLI-launched dialog binary built on Tauri (Rust + system WebView: WKWebView on macOS, WebView2 on Windows).
- Implements swiftDialog's CLI contract (~132 options) with a **tiered compatibility model**:
  - Tier 1 (v1): core content, icons/branding, buttons, user input, list/progress, command-file IPC, basic window behavior (~78 options).
  - Tier 2 (v1.x): blur screen, banners, media (image/video/webcontent), help sheet, advanced window options.
  - Tier 3: macOS-only system integrations (notification, dock icon, loginwindow) — accepted and no-op'd on Windows.
  - Tier 4: meta/cruft (builder, coffee, jh, listfonts) — stubbed or dropped.
- **Degrade, never break**: the parser accepts all swiftDialog options from day one; unimplemented or platform-unavailable options (e.g. `--blurscreen` on Windows) log a warning and no-op. A script must still show its dialog.
- **Bit-compatible scripting contract**: exit codes (button1=0, button2=2, info=3, timer=4, …) and JSON output structure match swiftDialog exactly.
- Live updates via swiftDialog's command-file mechanism (file watching; default path maps to the platform temp dir on Windows).
- Icon resolution per platform: file path and URL everywhere; `SF=name` maps to Segoe Fluent Icons on Windows via a name-mapping table with user-supplied override packs; app-bundle icon extraction maps to .exe/.lnk icon extraction.
- **User context only**: the dialog always runs in the logged-in user's session, like swiftDialog. Privileged processes drive it through the command file. A helper service that launches the dialog into an active user session is explicitly **out of scope** for this change (future work).

## Capabilities

### New Capabilities
- `cli-compatibility`: Full swiftDialog option parsing, tier model, degrade-don't-break semantics, exit-code and JSON-output contract.
- `dialog-core`: Core dialog rendering — title, subtitle, markdown message, layout presets (small/big/mini/presentation/style), fonts, sizing.
- `icon-branding`: Icon resolution (path/URL/system-symbol/app-icon extraction), overlay icons, banner image/title, built-in warning/info/caution icons.
- `buttons-actions`: Button 1/2/info text, actions (open URL), symbols, enable/disable, styles, and the button-to-exit-code mapping.
- `user-input`: Textfields (secure/required/regex/prompt), checkboxes/switches, dropdown selects, and their JSON output.
- `list-progress`: List items with per-row status and progress, overall progress bar, progress text, timer with optional bar.
- `command-file-ipc`: Live dialog updates via watched command file — the 50+ runtime update commands (list updates, progress, message, button state, quit, …).
- `window-behavior`: Positioning, ontop, fullscreen, moveable, resizable, appearance (light/dark), multi-screen; blur screen as best-effort/Tier 2 (acceptable to be unavailable on Windows).

### Modified Capabilities

(none — greenfield project, no existing specs)

## Impact

- New codebase in this repo (currently empty apart from research material in `.ext/`).
- Dependencies: Tauri/Rust toolchain; WebView2 runtime on Windows (preinstalled on Win11; bootstrapper needed for older fleets); no bundled browser.
- Reference sources: `.ext/swiftDialog` (contract source of truth — `CommandLineArguments.swift`, command-file processing, JSON output), swiftdialog.app docs.
- Out of scope: SYSTEM-context helper service, notification subsystem, ConstructionKit-style builder, jamfHelper compat mode.
- Risk: compat fidelity is a moving target (swiftDialog is actively developed); pin compatibility to a specific swiftDialog version (current: v3.0.1) and document deviations.
