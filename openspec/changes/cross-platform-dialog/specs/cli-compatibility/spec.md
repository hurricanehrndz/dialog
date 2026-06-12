# cli-compatibility

Drop-in compatibility with swiftDialog's command-line contract: option parsing, exit codes, and stdout output. Normative reference: `.ext/swiftDialog/dialog/Command Line/CommandLineArguments.swift` and `Functions/System.swift` (v2.5.x-era).

## ADDED Requirements

### Requirement: Parse the full swiftDialog option surface
The binary SHALL accept every swiftDialog CLI option (long form and documented short aliases, e.g. `--title`/`-t`, `--message`/`-m`, `--icon`/`-i`), as declared in a single option table mirroring upstream. Parsing an option present in the table SHALL never produce a fatal "unknown option" error.

#### Scenario: Known but unimplemented option degrades gracefully
- **WHEN** the binary is invoked with `--title "Hi" --message "There" --blurscreen` on a platform where blur is not implemented
- **THEN** a warning naming `--blurscreen` is written to stderr, the option is ignored, and the dialog is still displayed with the given title and message

#### Scenario: Deprecated alias accepted
- **WHEN** the binary is invoked with `--alignment center` (deprecated alias of `--messagealignment`)
- **THEN** the message alignment is set to center with no error

#### Scenario: Truly unknown option
- **WHEN** the binary is invoked with an option absent from the swiftDialog contract (e.g. `--frobnicate --title Test`)
- **THEN** the unknown token is ignored and the dialog is shown normally, matching upstream (verified live against swiftDialog 3.0.1: `--frobnicate` + timer exits 4 with no error output); the ignored token is reported on stderr only under `--verbose`

### Requirement: swiftDialog exit code contract
The binary SHALL exit with swiftDialog's exact exit codes: `0` button1/normal exit, `2` button2, `3` info button (when `--quitoninfo`), `4` timer expired, `5` quit via command file `quit:`, `10` quit via quitkey, `15` window close button, `20` "Timeout Exceeded", `30` "Key authorisation required" (`--key` mismatch), `40` SIGTERM received, `201` image resource not found, `202` file not found (e.g. bad `--jsonfile`), `203` invalid colour value, `255` forced immediate exit. (Reference: `AppVariables.swift:53-74`.)

#### Scenario: Button 1 exit
- **WHEN** the user clicks button 1
- **THEN** the process exits with code 0

#### Scenario: Button 2 exit
- **WHEN** the user clicks button 2
- **THEN** the process exits with code 2

#### Scenario: Timer expiry exit
- **WHEN** `--timer 2` is given and no interaction occurs for 2 seconds
- **THEN** the process exits with code 4

#### Scenario: Missing JSON file
- **WHEN** invoked with `--jsonfile /nonexistent.json`
- **THEN** the process exits with code 202 and an error message naming the file

### Requirement: JSON and key-value output contract
On exit with user-input elements present, the binary SHALL print results to stdout exactly as swiftDialog does. Default output is `key : value` lines; with `--json`, output is a single JSON object. Key naming SHALL match upstream: textfields output `{"<name>": "<value>"}`; each select outputs `{"<name>": {"selectedValue": "...", "selectedIndex": N}}` (index `-1` if unmatched), and a single-select dialog additionally outputs legacy keys `"SelectedOption"` (string) and `"SelectedIndex"` (integer); checkboxes output `{"<name>": <bool>}`; with `--enablelistselect`, list items output `{"<title>": <bool>}`. (Reference: `Functions/System.swift:280-380`.)

#### Scenario: Textfield JSON output
- **WHEN** invoked with `--textfield "Computer Name" --json` and the user enters "kraken" and clicks button 1
- **THEN** stdout contains a JSON object where key `Computer Name` has string value `kraken`, and the exit code is 0

#### Scenario: Single select legacy keys
- **WHEN** invoked with `--selecttitle "Dept" --selectvalues "IT,HR" --json` and the user selects "HR"
- **THEN** the JSON output contains `"SelectedOption": "HR"`, `"SelectedIndex": 1`, and `"Dept": {"selectedValue": "HR", "selectedIndex": 1}`

#### Scenario: No input elements, no output
- **WHEN** invoked with only `--title` and `--message` and the user clicks button 1
- **THEN** stdout contains no key-value output

### Requirement: JSON input
The binary SHALL accept full dialog configuration via `--jsonfile <path>` and `--jsonstring <json>`, where keys are the long option names (without `--`) and values follow swiftDialog's JSON input schema, including array forms for `textfield`, `checkbox`, `selectitems`, and `listitem`.

#### Scenario: jsonstring drives the dialog
- **WHEN** invoked with `--jsonstring '{"title": "Hello", "message": "World", "button1text": "Done"}'`
- **THEN** the dialog displays title "Hello", message "World", and a button labelled "Done"

### Requirement: stdout reaches the invoking console on Windows
On Windows the binary SHALL attach to the parent process console so that stdout (JSON output) and stderr (warnings) are visible to the invoking PowerShell/cmd session and capturable via redirection, while not flashing a console window when launched outside one.

#### Scenario: PowerShell captures JSON output
- **WHEN** a PowerShell script runs `$result = & dialog.exe --textfield "Name" --json | Out-String` and the user submits the form
- **THEN** `$result` contains the JSON output string
- (Note, verified on the rig: plain `$x = & gui.exe` is fire-and-forget for GUI-subsystem executables — a PowerShell semantic affecting all GUI apps. Any pipeline stage forces wait-and-capture; bare runs print to the console via conditional AttachConsole.)

### Requirement: Version and help output
`--version` SHALL print the application version to stdout and exit 0. `--help` SHALL print usage covering the implemented option set and exit 0. `--verbose` SHALL enable debug logging to stderr.

#### Scenario: Version flag
- **WHEN** invoked with `--version`
- **THEN** a semantic version string is printed to stdout and the process exits 0
