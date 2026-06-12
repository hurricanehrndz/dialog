# list-progress

The list-item status display and progress reporting — swiftDialog's signature deployment-UI feature (Setup-Your-Mac-style onboarding screens).

## ADDED Requirements

### Requirement: List items
`--listitem <spec>` SHALL be repeatable, accepting `"<Title>[,status=<status>][,statustext=<text>][,icon=<icon>][,subtitle=<text>]"`, rendering a scrollable list of rows each with title, optional icon, status indicator, and status text. JSON input via the `listitem` array SHALL be equivalent.

#### Scenario: Static list rendered
- **WHEN** invoked with `--listitem "Install Chrome" --listitem "Configure Wi-Fi" --listitem "Enroll Device"`
- **THEN** the dialog shows a list of three rows, each with the given title and an empty status

### Requirement: List item status values
Each list item SHALL support the status values `wait` (animated spinner), `progress` (per-row progress bar), `success`, `fail`, `error`, `pending`, and `none`, rendered with platform-appropriate status glyphs. (Reference: `Views/ListView.swift:190-205`.)

#### Scenario: Status glyphs render
- **WHEN** a list item has `status: success` and another has `status: fail`
- **THEN** the first shows a success (checkmark) indicator and the second an error (cross) indicator

#### Scenario: Wait status animates
- **WHEN** a list item has `status: wait`
- **THEN** an indeterminate spinner renders in that row's status area

### Requirement: Live list updates via command file
List items SHALL be addressable at runtime through the command file using swiftDialog's `listitem:` verb forms: by title (`listitem: title: <title>, status: <status>, statustext: <text>`), by index (`listitem: index: <n>, ...`), `add:`/`delete:` to mutate rows, `progress: <0-100>` for per-row progress, and `list: <a,b,c>` to replace the whole list (with `list: clear` emptying it).

#### Scenario: Update by title
- **WHEN** a dialog with list item "Install Chrome" is running and `listitem: title: Install Chrome, status: success, statustext: Done` is appended to the command file
- **THEN** that row shows the success glyph and "Done" within 1 second

#### Scenario: Append a new row
- **WHEN** `listitem: add, title: Install Slack, status: wait` is appended to the command file
- **THEN** a new row "Install Slack" with a spinner appears at the end of the list

#### Scenario: Replace entire list
- **WHEN** `list: Step 1, Step 2, Step 3` is appended to the command file
- **THEN** the list now contains exactly those three rows

### Requirement: Overall progress bar
`--progress [n]` SHALL render a determinate progress bar of `n` total steps (indeterminate when no value), with `--progresstext <text>` rendering a status line beneath it. Command-file updates SHALL support `progress: <value>`, `progress: increment`, `progress: reset`, `progress: indeterminate`, `progress: complete`, and `progresstext: <text>`. (Reference: `DialogUpdatableContent.swift:313-339`.)

#### Scenario: Progress increments
- **WHEN** a dialog launched with `--progress 4` receives `progress: increment` twice via the command file
- **THEN** the progress bar shows 2 of 4 complete

#### Scenario: Progress completes
- **WHEN** `progress: complete` is appended to the command file
- **THEN** the progress bar fills to 100%

### Requirement: Selectable lists
With `--enablelistselect`, list rows SHALL be user-toggleable, and each row SHALL contribute `"<title>" : "<true|false>"` to the exit output per the cli-compatibility contract.

#### Scenario: List selection output
- **WHEN** invoked with `--listitem "Optional App A" --listitem "Optional App B" --enablelistselect --json` and the user selects only "Optional App A" and clicks button 1
- **THEN** the JSON output contains `"Optional App A": true` and `"Optional App B": false`
