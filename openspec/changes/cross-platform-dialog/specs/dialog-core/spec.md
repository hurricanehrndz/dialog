# dialog-core

Core dialog window and content rendering: title, subtitle, message with markdown, fonts, sizing presets, and appearance.

## ADDED Requirements

### Requirement: Basic dialog content
The binary SHALL display a dialog with a title (`--title`, default "An Important Message"), optional subtitle (`--subtitle`), and message body (`--message`). `--title none` SHALL hide the title area. The default window size SHALL be 820×380 points, overridable via `--width` and `--height`. (Reference: `AppVariables.swift:103-104`.)

#### Scenario: Minimal invocation
- **WHEN** invoked with `--title "Update Required" --message "Please save your work."`
- **THEN** a centered dialog of 820×380 appears showing the title and message with a default icon and an "OK" button

#### Scenario: Custom size
- **WHEN** invoked with `--width 500 --height 300`
- **THEN** the dialog window is 500×300

### Requirement: Markdown message rendering
The message body SHALL render CommonMark markdown: bold, italic, headings, ordered/unordered lists, hyperlinks (opening in the default browser), inline code, and images. Plain text without markdown syntax SHALL render unchanged.

#### Scenario: Markdown formatting
- **WHEN** the message is `"**Important:** read the [policy](https://example.com)"`
- **THEN** "Important:" renders bold and "policy" renders as a clickable link that opens https://example.com in the default browser

### Requirement: Text styling and alignment
The binary SHALL support `--titlefont` and `--messagefont` accepting swiftDialog's comma-separated `name=,size=,weight=,colour=/color=` syntax, `--messagealignment` (`left|centre|center|right`), and `--messageposition` (`top|centre|center|bottom`).

#### Scenario: Title font styling
- **WHEN** invoked with `--titlefont "size=30,colour=#FF0000"`
- **THEN** the title renders at 30 points in red

### Requirement: Window size presets
The binary SHALL support `--small` (compact ~0.75 scale), `--big` (large ~1.25 scale), `--mini` (minimal fixed-height progress-style dialog), and `--style <presentation|mini|centred|centered|alert|caution|warning>` layout presets matching swiftDialog's layouts in intent.

#### Scenario: Mini mode
- **WHEN** invoked with `--mini --title "Installing" --progress`
- **THEN** a compact dialog appears with title, progress bar, and no message area

#### Scenario: Alert style
- **WHEN** invoked with `--style alert --message "Disk almost full"`
- **THEN** a small centered alert-format dialog appears with centered icon and message

### Requirement: Appearance modes
The binary SHALL follow the OS light/dark appearance by default and SHALL support `--appearance <light|dark>` to force a mode. On macOS the dialog SHALL use macOS-style typography and controls; on Windows it SHALL use Fluent/Windows 11-style typography and controls.

#### Scenario: Forced dark mode
- **WHEN** invoked with `--appearance dark` on a system in light mode
- **THEN** the dialog renders with the dark theme

### Requirement: Background image
The binary SHALL support `--background <path>` with `--bgalpha`, `--bgposition`, `--bgfill`/`--bgscale` to display a window background image behind dialog content.

#### Scenario: Background image displayed
- **WHEN** invoked with `--background /path/wallpaper.png --bgalpha 0.5`
- **THEN** the image renders behind the dialog content at 50% opacity
