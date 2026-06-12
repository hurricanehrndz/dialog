# window-behavior

Window placement, layering, chrome, and screen effects. Several effects are best-effort on Windows per the degrade-don't-break rule.

## ADDED Requirements

### Requirement: Window positioning
`--position <topleft|left|bottomleft|top|center|centre|bottom|topright|right|bottomright>` SHALL place the window in the named screen region (default centered), and `--positionoffset <points>` SHALL inset it from screen edges.

#### Scenario: Top-right placement
- **WHEN** invoked with `--position topright --positionoffset 20`
- **THEN** the window appears 20 points from the top and right edges of the primary screen

### Requirement: Window layering and chrome
`--ontop` SHALL keep the dialog above normal windows. `--moveable` SHALL allow dragging (windows are not moveable by default). `--resizable` SHALL allow resizing. `--windowbuttons <min,max,close>` SHALL control which title-bar buttons are present; by default the dialog has no close/minimize controls. Closing via an enabled window close button SHALL exit with code 15.

#### Scenario: Always on top
- **WHEN** invoked with `--ontop` and another application window is focused
- **THEN** the dialog remains visible above it

#### Scenario: Window close button exit code
- **WHEN** invoked with `--windowbuttons close` and the user clicks the close button
- **THEN** the process exits with code 15

### Requirement: Fullscreen and blur effects
`--fullscreen` SHALL display the dialog content over a full-screen backdrop. `--blurscreen` SHALL blur (or, where blur is unavailable, dim) everything behind the dialog on all displays when combined with `--showonallscreens`. On Windows, if compositor blur is unavailable, the binary SHALL degrade to a dimmed overlay and log a warning — the dialog MUST still display.

#### Scenario: Blur degrades on Windows
- **WHEN** invoked on Windows with `--blurscreen` and compositor blur is unavailable
- **THEN** a dimmed full-screen overlay renders behind the dialog, a warning is logged to stderr, and the dialog functions normally

#### Scenario: Fullscreen mode
- **WHEN** invoked with `--fullscreen --title "Welcome" --message "Setting up your device"`
- **THEN** the content renders over a backdrop covering the entire primary display

### Requirement: Multi-display awareness
`--showonallscreens` SHALL render the backdrop/blur overlay on every attached display while showing the interactive dialog on the primary (or focused) display.

#### Scenario: Overlay on every display
- **WHEN** invoked with `--blurscreen --showonallscreens` on a two-display system
- **THEN** both displays are covered by the overlay and the dialog appears on one

### Requirement: Single dialog instance behavior
Launching the binary while another instance is showing SHALL behave like swiftDialog: each invocation is an independent process and window; no implicit singleton. The `activate:` command verb brings a specific instance forward via its own command file.

#### Scenario: Two independent dialogs
- **WHEN** two invocations run concurrently with different `--commandfile` paths
- **THEN** two dialogs are shown and each responds only to its own command file
