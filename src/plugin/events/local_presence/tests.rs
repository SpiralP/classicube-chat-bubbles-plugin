use super::{display_for_input, format_input_line, is_sensitive_text};

/// Default ClassiCube palette covers '0'..='9', 'a'..='f', 'A'..='F'.
fn default_palette(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

#[test]
fn percent_to_amp_for_valid_code() {
    assert_eq!(
        format_input_line("%chello world", true, default_palette),
        "&chello world"
    );
}

#[test]
fn percent_left_alone_for_invalid_code() {
    // 'z' is not in the default palette.
    assert_eq!(format_input_line("%zfoo", true, default_palette), "%zfoo");
}

#[test]
fn trailing_percent_at_end_of_string() {
    // No byte follows '%', so it must be preserved.
    assert_eq!(format_input_line("done%", true, default_palette), "done%");
}

#[test]
fn multiple_codes_in_one_string() {
    assert_eq!(
        format_input_line("%ared %bgreen %cblue", true, default_palette),
        "&ared &bgreen &cblue"
    );
}

#[test]
fn convert_percents_off_is_identity() {
    // Classic mode: widget sets convertPercents = false, raw stays raw.
    assert_eq!(
        format_input_line("%chello", false, default_palette),
        "%chello"
    );
}

#[test]
fn empty_string_is_empty() {
    assert_eq!(format_input_line("", true, default_palette), "");
    assert_eq!(format_input_line("", false, default_palette), "");
}

#[test]
fn ampersand_passthrough() {
    // Already-formatted text from the post-send path must not be rewritten.
    assert_eq!(
        format_input_line("&chello", true, default_palette),
        "&chello"
    );
}

#[test]
fn non_ascii_passes_through() {
    // UTF-8 multibyte chars should round-trip; '%' cannot appear inside
    // a codepoint (it's ASCII 0x25, never a continuation byte).
    assert_eq!(
        format_input_line("héllo %cwörld", true, default_palette),
        "héllo &cwörld"
    );
}

#[test]
fn adjacent_percent_signs() {
    // First '%' is followed by '%' (not a valid code), so it stays;
    // second '%' is followed by 'c' (valid), so it converts.
    assert_eq!(
        format_input_line("%%chello", true, default_palette),
        "%&chello"
    );
}

#[test]
fn empty_palette_never_converts() {
    // Mirrors ClassiCube startup before palette init (or all colors zero).
    assert_eq!(format_input_line("%chello", true, |_| false), "%chello");
}

#[test]
fn display_for_input_hides_on_empty() {
    // Erasing everything hides the bubble, even in whisper-mode.
    assert_eq!(display_for_input("", false), "");
    assert_eq!(display_for_input("", true), "");
}

#[test]
fn display_for_input_masks_private_messages() {
    assert_eq!(display_for_input("/help", false), "...");
    assert_eq!(display_for_input("@SpiralP hi", false), "...");
    assert_eq!(display_for_input("##secret", false), "...");
    assert_eq!(display_for_input("++admin", false), "...");
    assert_eq!(display_for_input("#", false), "...");
    assert_eq!(display_for_input("+", false), "...");
}

#[test]
fn display_for_input_masks_everything_in_whisper_mode() {
    // In whisper-mode the whole line is private, so even plain text masks.
    assert_eq!(display_for_input("hello", true), "...");
}

#[test]
fn display_for_input_shows_normal_text_verbatim() {
    assert_eq!(display_for_input("hello", false), "hello");
    assert_eq!(display_for_input("#single", false), "#single");
    assert_eq!(display_for_input("+single", false), "+single");
}

#[test]
fn is_sensitive_text_filters_whispers_and_commands() {
    assert!(is_sensitive_text("@SpiralP hi"));
    assert!(is_sensitive_text("/help"));
    assert!(is_sensitive_text("##secret"));
    assert!(is_sensitive_text("++admin"));
    assert!(!is_sensitive_text("hello"));
    assert!(!is_sensitive_text(""));
    assert!(!is_sensitive_text("#single"));
    assert!(!is_sensitive_text("+single"));
    assert!(is_sensitive_text("#"));
    assert!(is_sensitive_text("+"));
}
