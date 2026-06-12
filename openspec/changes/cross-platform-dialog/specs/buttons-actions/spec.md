# buttons-actions

Button rendering, actions, keyboard behavior, and the button-to-exit-code mapping that scripts depend on.

## ADDED Requirements

### Requirement: Button 1 (default action)
Button 1 SHALL always be present with default label "OK" (`--button1text` to override), act as the default keyboard action (Return/Enter), and exit with code 0 when clicked. `--button1action <url>` SHALL open the URL in the default browser on click before exiting. `--button1disabled` SHALL render it disabled until enabled via the command file.

#### Scenario: Default button text and exit
- **WHEN** invoked with no button options and the user presses Return
- **THEN** button 1 (labelled "OK") activates and the process exits 0

#### Scenario: Button 1 action URL
- **WHEN** invoked with `--button1action "https://example.com"` and the user clicks button 1
- **THEN** the URL opens in the default browser and the process exits 0

#### Scenario: Disabled until enabled
- **WHEN** invoked with `--button1disabled`
- **THEN** button 1 renders disabled and cannot be activated by mouse or keyboard

### Requirement: Button 2 (cancel)
`--button2` SHALL show a second button with default label "Cancel"; `--button2text` overrides the label and implies its presence. Button 2 SHALL act as the keyboard cancel action (Escape) and exit with code 2. `--button2action` and `--button2disabled` SHALL behave analogously to button 1.

#### Scenario: Cancel via Escape
- **WHEN** invoked with `--button2` and the user presses Escape
- **THEN** the process exits with code 2

### Requirement: Info button
`--infobuttontext <label>` (or `--infobutton`) SHALL show a tertiary button in the lower-leading corner. With `--infobuttonaction <url>`, clicking opens the URL without quitting; with `--quitoninfo`, clicking exits with code 3.

#### Scenario: Info button quits with code 3
- **WHEN** invoked with `--infobuttontext "More Info" --quitoninfo` and the user clicks it
- **THEN** the process exits with code 3

#### Scenario: Info button opens URL without quitting
- **WHEN** invoked with `--infobuttontext "Help" --infobuttonaction "https://help.example.com"` and the user clicks it
- **THEN** the URL opens and the dialog remains open

### Requirement: Button styling
The binary SHALL support `--buttonstyle <center|centre|stack>` layout variants, `--buttonsize <mini|small|regular|large>`, and button symbols (`--button1symbol` etc.) resolved through the same system-symbol pipeline as icons.

#### Scenario: Stacked buttons
- **WHEN** invoked with `--buttonstyle stack --button2`
- **THEN** buttons render stacked vertically at full width

### Requirement: Timer with optional bar
`--timer [seconds]` SHALL auto-dismiss the dialog after the given duration (default 10 seconds when no value supplied), exiting with code 4. A countdown bar SHALL be shown unless `--hidetimerbar` is given. (Reference: `AppVariables.swift:44`.)

#### Scenario: Timer default duration
- **WHEN** invoked with `--timer` and no interaction occurs
- **THEN** the dialog dismisses after 10 seconds with exit code 4

#### Scenario: Hidden timer bar
- **WHEN** invoked with `--timer 30 --hidetimerbar`
- **THEN** no countdown bar is visible but the dialog still exits with code 4 after 30 seconds

### Requirement: Quit key
`--quitkey <character>` SHALL register Cmd+<char> (macOS) / Ctrl+<char> (Windows) to quit the dialog with exit code 10. The default quit key behavior (Cmd/Ctrl+Q) SHALL also exit 10.

#### Scenario: Custom quit key
- **WHEN** invoked with `--quitkey x` and the user presses Cmd+X (macOS) or Ctrl+X (Windows)
- **THEN** the process exits with code 10
