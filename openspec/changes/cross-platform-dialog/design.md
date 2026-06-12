## Context

swiftDialog (`.ext/swiftDialog`, ~65k lines Swift/SwiftUI, 132 CLI options) is macOS-only. We are building a drop-in-compatible reimplementation for Windows + macOS. The repo is greenfield; `.ext/swiftDialog` is the normative contract reference (pinned at v3.0.1, the current release) and `.ext/zero-native` was evaluated and rejected. Research confirmed no competing cross-platform implementation exists.

Primary consumers are endpoint-management scripts (Jamf, Intune, SCCM): they launch the binary with flags, optionally stream commands into a watched file, and consume the exit code and stdout JSON. The dialog always runs in the logged-in user's session; privileged processes communicate only via the command file.

## Goals / Non-Goals

**Goals:**
- One binary per platform, drop-in compatible with swiftDialog invocations: same flags, same exit codes, same JSON output, same command-file protocol.
- Degrade-don't-break: every swiftDialog option parses; unimplemented/unavailable ones warn and no-op.
- Fast cold start (dialog visible well under 1s) and small distribution suitable for MDM push.
- Platform-appropriate appearance: macOS-styled on macOS, Fluent-styled on Windows.

**Non-Goals:**
- 100% feature parity. Tier 3 (notifications, dock icon, loginwindow) no-ops on Windows or both; Tier 4 (builder, coffee, jh, listfonts) is dropped/stubbed.
- SYSTEM-context UI. No service/agent in this change; a session-launching helper is future work.
- Pixel-identical rendering with swiftDialog. Layout intent and behavior are the contract, not pixels.
- Linux support (the stack permits it later; not designed for now).

## Decisions

### D1: Tauri 2.x (Rust + system WebView) as the application framework
WKWebView on macOS, WebView2 on Windows. Chosen over:
- **Flutter**: 25–60 MB bundles, GUI-subsystem stdout problem on Windows needs runner patching, imitation widgets anyway, heavier cold start. Tauri gives 3–10 MB binaries and first-class CLI ergonomics from Rust.
- **zero-native**: same architecture as Tauri but pre-release (v0.2.0), Windows path unproven (all examples are macOS/Linux). Betting the product and the framework simultaneously is too much risk.
- **Electron**: size and startup are disqualifying for a "pop a dialog from a script" tool.

Trade-off accepted: WebView2 runtime dependency on Windows (preinstalled Win11; document/bundle the bootstrapper for older fleets), and web-rendered "native-looking" UI rather than true native controls.

### D2: Rust core owns the contract; webview is a pure renderer
All scripting-contract surfaces live in Rust: CLI parsing, JSON input/output, exit codes, command-file watching, icon resolution, validation. The webview receives a single declarative `DialogState` model and emits semantic UI events (`button1-clicked`, `input-changed`, `listselect-toggled`). Rationale: the contract must be testable without a display, and the renderer must be swappable/restartable without touching compat behavior.

```
argv / --jsonstring ──▶ ┌─────────────┐   state (full+patch)  ┌──────────┐
command file (watch) ──▶│  Rust core   │ ─────────────────────▶│ WebView  │
                        │ DialogState  │ ◀───────────────────── │ renderer │
stdout JSON + exit  ◀── └─────────────┘   semantic UI events   └──────────┘
```

### D3: Option table is data, not code
The 132 options are declared in a single table (name, aliases, type, default, tier, platform availability) mirroring `CommandLineArguments.swift`. The parser accepts everything in the table; options whose tier/platform isn't implemented log `WARNING: --<opt> not supported (<reason>), ignoring` to stderr and no-op. This makes "degrade, never break" structural rather than per-feature discipline, and makes compat drift auditable by diffing tables against upstream.

### D4: Command-file watcher in Rust with seek-offset tailing
Watch via the `notify` crate with a polling fallback; read newly appended lines from a remembered offset, handle truncation by resetting to zero (matching swiftDialog's tail behavior). Default path: `/var/tmp/dialog.log` on macOS (unchanged) and `%TEMP%\dialog.log` resolved system-wide as `C:\Windows\Temp\dialog.log` is wrong for user context — use `%PUBLIC%\dialog.log` or `C:\ProgramData\dialog\dialog.log` with a permissive ACL so SYSTEM services can write and the user app can read. Exact Windows default is Open Question Q2. File is created world-writable (0666 on macOS, equivalent ACL on Windows) like swiftDialog.

### D5: One frontend, two themes
A single HTML/CSS/TS frontend (no heavy framework; Svelte or vanilla + lit-style components — final pick at implementation) with a platform theme switch: macOS design tokens (system font, vibrancy-like surfaces, macOS button order/styles) vs Fluent tokens (Segoe UI Variable, Win11 corner radii, Fluent buttons). Markdown rendered in the webview with a CommonMark library configured to match swift-markdown-ui's practical behavior (links, bold/italic, headings, lists, images).

### D6: Icon resolution as a per-platform Rust module
Accepted forms (compat): file path, URL (with `--checksum` verification), `SF=name` with color/palette modifiers, app path, builtin keywords (`warningicon`, `infoicon`, `cautionicon`, `default`, `none`).
- macOS: SF Symbols rendered natively where possible; `.app` icon via bundle lookup.
- Windows: `SF=name` maps through a curated SF→Segoe Fluent Icons table; unmapped names fall back to a generic glyph + warning. `.exe`/`.lnk` icon extraction via `SHGetFileInfo`/`IShellItemImageFactory`. Users can supply an override pack (directory or zip of `name.png/svg`) consulted before the builtin table.

### D7: Windows stdout handling
Build as GUI-subsystem (no console flash) and call `AttachConsole(ATTACH_PARENT_PROCESS)` at startup so stdout/stderr reach the invoking PowerShell/cmd session — required for the JSON output contract. Covered by an integration test on Windows CI.

### D8: Compat verification harness
A test suite of recorded swiftDialog invocations (flags → expected exit code + stdout, command-file scripts → expected state transitions) run against our binary. On macOS CI it can also run real swiftDialog side-by-side to detect upstream drift. The harness, not code review, is the definition of "drop-in compatible".

## Risks / Trade-offs

- [WebView2 missing/broken on managed Win10 fleets] → Detect at startup; clear stderr error + documented bootstrapper deployment guidance; exit with a distinct error code.
- [Webview cold start slower than native swiftDialog] → Measure from day one; keep frontend dependency-free and inline assets; target <500 ms to visible window on reference hardware.
- [Compat drift as upstream swiftDialog evolves] → Pin contract to v3.0.1; option-table diffing + harness runs against new upstream releases; document deviations in a COMPATIBILITY.md.
- [SF Symbol mapping will never be complete] (~6,000 symbols) → Curate the ~200 symbols actually seen in community scripts; override packs for the rest; warn-and-fallback otherwise.
- [Blur/fullscreen overlay fidelity on Windows] → Accepted: best-effort Tier 2; degrade to dimmed overlay or plain ontop window with a warning.
- [Markdown rendering differences vs swift-markdown-ui] → Accepted: CommonMark-conformant is the bar; document known differences.
- [Two themes ≠ native controls; sharp-eyed users will notice] → Accepted trade-off of D1; prioritize typography, spacing, and button order fidelity which carry most of the perceived nativeness.

## Migration Plan

Greenfield — no migration. Delivery is staged by tier (Tier 1 capabilities first), with the compat harness gating each milestone. Rollback is per-release binary replacement; no persistent state beyond the command file.

## Open Questions

- Q1: Frontend layer — Svelte vs vanilla TS components (decide at first implementation task; constraint: zero network deps, inline bundle).
- Q2: Windows default command-file path and ACL model (`%PUBLIC%`, `ProgramData`, or per-session temp + `--commandfile` always-explicit guidance).
- Q3: Whether macOS rendering of `SF=` should use true SF Symbols via native swift shim or pre-rendered assets (fidelity vs build complexity).
- Q4: Distribution/signing: MSI vs portable exe + notarized pkg/dmg; decide before first public release, not needed for v1 development.
