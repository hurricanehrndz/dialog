# user-input

Form elements — textfields, checkboxes/switches, and dropdown selects — including required-field validation and their contribution to the output contract.

## ADDED Requirements

### Requirement: Textfields
`--textfield <spec>` SHALL be repeatable and accept swiftDialog's comma-separated sub-option syntax: `"<Title>[,required][,secure][,prompt=<text>][,value=<default>][,regex=<pattern>][,regexerror=<message>][,name=<key>]"`. Secure fields mask input. On submit, each textfield contributes `<name-or-title> : <value>` to the output per the cli-compatibility contract.

#### Scenario: Plain textfield round-trip
- **WHEN** invoked with `--textfield "Asset Tag"` and the user enters "A-1234" and clicks button 1
- **THEN** stdout contains `Asset Tag : A-1234` and the exit code is 0

#### Scenario: Secure field masks input
- **WHEN** invoked with `--textfield "Password,secure"`
- **THEN** typed characters are masked in the UI

#### Scenario: Required field blocks submit
- **WHEN** invoked with `--textfield "Email,required"` and the user clicks button 1 with the field empty
- **THEN** the dialog does not exit, the field is highlighted, and an explanatory message is shown

#### Scenario: Regex validation
- **WHEN** invoked with `--textfield "Serial,regex=^[A-Z0-9]{10}$,regexerror=Must be 10 characters"` and the user submits "abc"
- **THEN** the dialog does not exit and "Must be 10 characters" is shown

### Requirement: Checkboxes and switches
`--checkbox <spec>` SHALL be repeatable with syntax `"<Label>[,checked][,disabled][,name=<key>]"`. `--checkboxstyle <checkbox|switch>` SHALL select the control rendering. On submit, each contributes `"<name-or-label>" : "<true|false>"` to the output.

#### Scenario: Checkbox state in output
- **WHEN** invoked with `--checkbox "Enable FileVault,checked" --checkbox "Join Wi-Fi" --json` and the user unchecks nothing and clicks button 1
- **THEN** the JSON output contains `"Enable FileVault": true` and `"Join Wi-Fi": false`

#### Scenario: Switch style rendering
- **WHEN** invoked with `--checkbox "Telemetry" --checkboxstyle switch`
- **THEN** the control renders as a toggle switch

### Requirement: Dropdown selects
`--selecttitle <title>[,required][,radio]` with `--selectvalues "<a,b,c>"` and optional `--selectdefault <value>` SHALL render a labelled dropdown (or radio group with the `radio` modifier). Multiple select groups SHALL be supported via repeated `--selecttitle`/`--selectvalues` pairs and via JSON `selectitems`. Output SHALL follow the cli-compatibility select contract including legacy `SelectedOption`/`SelectedIndex` keys for the single-select case.

#### Scenario: Select round-trip
- **WHEN** invoked with `--selecttitle "Department" --selectvalues "Engineering,Sales,HR" --selectdefault "Sales"` and the user keeps the default and clicks button 1
- **THEN** stdout contains `"Department" : "Sales"` and `"Department" index : "1"`

#### Scenario: Required select blocks submit
- **WHEN** invoked with `--selecttitle "Site,required" --selectvalues "NYC,SFO"` and the user clicks button 1 without selecting
- **THEN** the dialog does not exit and the select is highlighted as required

### Requirement: Input returned on non-default exits when requested
With `--alwaysreturninput`, the binary SHALL print collected input values on any exit path (button 2, timer) rather than only on button 1.

#### Scenario: Timer expiry returns input
- **WHEN** invoked with `--textfield "Name" --alwaysreturninput --timer 2` and the user types "bob" but does not click before expiry
- **THEN** the process exits with code 4 and stdout contains `Name : bob`
