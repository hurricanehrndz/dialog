# command-file-ipc

The live-update mechanism: a watched command file through which any process (including privileged ones) drives a running dialog. This is the cross-privilege IPC boundary — the dialog runs in user context; writers may be SYSTEM/root.

## ADDED Requirements

### Requirement: Command file watching
The binary SHALL watch the file given by `--commandfile <path>` (default `/var/tmp/dialog.log` on macOS; a fixed world-writable default on Windows, documented and overridable). Newly appended lines SHALL be processed in order within 1 second of being written. Lines are processed from the binary's start offset; pre-existing content is ignored unless the file is created by the binary. On file truncation, the read offset SHALL reset to the start. The file SHALL be created world-writable (0666 / equivalent ACL) if absent, so privileged processes can write to it.

#### Scenario: Append is processed
- **WHEN** a dialog is running and `title: New Title` is appended to the command file
- **THEN** the dialog title changes to "New Title" within 1 second

#### Scenario: Privileged writer
- **WHEN** the dialog runs in a user session and a SYSTEM/root process appends `progresstext: Installing package 3 of 7` to the command file
- **THEN** the dialog updates the progress text

#### Scenario: Truncation handled
- **WHEN** the command file is truncated to zero length and a new command is then appended
- **THEN** the new command is processed normally

### Requirement: Content update verbs
The binary SHALL support swiftDialog's command verbs for content: `title:`, `subtitle:`, `message:` (with `+ <text>` append form), `icon:`, `overlayicon:`, `image:`, `infotext:`, `infobox:`, `progress:`, `progresstext:`, `listitem:`/`list:` (per list-progress spec), `webcontent:`, and `video:`. Verb names match the corresponding long option names. (Reference: `Updatable Content/DialogUpdatableContent.swift`.)

#### Scenario: Message replaced and appended
- **WHEN** `message: Step one complete` then `message: + And step two` are appended to the command file
- **THEN** the message shows "Step one complete" followed by "And step two"

#### Scenario: Icon swapped at runtime
- **WHEN** `icon: SF=checkmark.circle.fill` is appended to the command file
- **THEN** the dialog icon changes to the resolved checkmark symbol

### Requirement: Control verbs
The binary SHALL support control verbs: `button1text:`, `button2text:`, `button1: enable|disable`, `button2: enable|disable`, `quit:` (exit code 5), `activate:` (bring window to front), `width:`, `height:`, `position:`, and `quitkey:`.

#### Scenario: Enable button when work completes
- **WHEN** a dialog launched with `--button1disabled` receives `button1: enable` via the command file
- **THEN** button 1 becomes clickable

#### Scenario: Scripted quit
- **WHEN** `quit:` is appended to the command file
- **THEN** the process exits with code 5

#### Scenario: Window repositioned
- **WHEN** `position: topright` is appended to the command file
- **THEN** the window moves to the top-right of the screen

### Requirement: Malformed commands are ignored
Lines that do not parse as a known verb SHALL be ignored with a debug log entry; they SHALL NOT crash the dialog or stop processing of subsequent lines.

#### Scenario: Garbage line skipped
- **WHEN** `this is not a command` followed by `title: Still Works` are appended to the command file
- **THEN** the dialog ignores the first line and updates the title from the second
