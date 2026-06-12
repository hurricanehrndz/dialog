//! swiftDialog's exit code contract. Values from `AppVariables.swift:53-74`
//! (v3.0.1); scripts branch on these, so they are bit-for-bit normative.

/// Button 1 pressed / normal exit.
pub const BUTTON1: i32 = 0;
/// Button 2 pressed.
pub const BUTTON2: i32 = 2;
/// Info button pressed (with `--quitoninfo`).
pub const INFO_BUTTON: i32 = 3;
/// Timer expired.
pub const TIMER: i32 = 4;
/// `quit:` received via the command file.
pub const QUIT_COMMAND: i32 = 5;
/// Quit via quitkey (Cmd/Ctrl+<key>).
pub const QUIT_KEY: i32 = 10;
/// Window close button.
pub const WINDOW_CLOSE: i32 = 15;
/// "Timeout Exceeded".
pub const TIMEOUT: i32 = 20;
/// "Key authorisation required" — `--key` mismatch.
pub const KEY_AUTH_REQUIRED: i32 = 30;
/// SIGTERM received.
pub const SIGTERM: i32 = 40;
/// Image resource cannot be found.
pub const IMAGE_NOT_FOUND: i32 = 201;
/// File not found (e.g. `--jsonfile` path).
pub const FILE_NOT_FOUND: i32 = 202;
/// Invalid colour value specified.
pub const INVALID_COLOUR: i32 = 203;
/// Forced immediate exit.
pub const EXIT_NOW: i32 = 255;

/// stderr message that accompanies certain exit codes, exactly as upstream
/// prints them (`AppVariables.swift`).
pub fn exit_message(code: i32) -> Option<&'static str> {
    match code {
        TIMEOUT => Some("Timeout Exceeded"),
        KEY_AUTH_REQUIRED => Some("Key authorisation required"),
        IMAGE_NOT_FOUND => Some("ERROR: Image resource cannot be found :"),
        FILE_NOT_FOUND => Some("ERROR: File not found :"),
        INVALID_COLOUR => Some("ERROR: Invalid Colour Value Specified. Use format #000000 :"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole table, pinned against AppVariables.swift:53-74.
    #[test]
    fn codes_match_upstream() {
        assert_eq!(BUTTON1, 0);
        assert_eq!(BUTTON2, 2);
        assert_eq!(INFO_BUTTON, 3);
        assert_eq!(TIMER, 4);
        assert_eq!(QUIT_COMMAND, 5);
        assert_eq!(QUIT_KEY, 10);
        assert_eq!(WINDOW_CLOSE, 15);
        assert_eq!(TIMEOUT, 20);
        assert_eq!(KEY_AUTH_REQUIRED, 30);
        assert_eq!(SIGTERM, 40);
        assert_eq!(IMAGE_NOT_FOUND, 201);
        assert_eq!(FILE_NOT_FOUND, 202);
        assert_eq!(INVALID_COLOUR, 203);
        assert_eq!(EXIT_NOW, 255);
    }

    #[test]
    fn messages_match_upstream() {
        assert_eq!(
            exit_message(FILE_NOT_FOUND),
            Some("ERROR: File not found :")
        );
        assert_eq!(exit_message(BUTTON1), None);
    }
}
