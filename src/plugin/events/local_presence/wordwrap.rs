//! Port of MCGalaxy `LineWrapper.Wordwrap`
//! (`MCGalaxy/MCGalaxy/Chat/LineWrapper.cs:70-156`), operating on CP437 bytes
//! so the wrap boundary matches the server's exactly.
//!
//! Differences from MCGalaxy:
//! - `supportsEmotes` is hardcoded to `true`: ClassiCube supports emotes and
//!   our renderer is Drawer2D, which doesn't trim trailing emotes (that's a
//!   workaround for original Minecraft Classic's glyph trim). Both the
//!   first-line emote-pad branch and the `EndsInEmote` adjustment are skipped.
//! - `CleanupColors` is not run before wrapping. Input with adjacent color
//!   codes (e.g. `&a&bhello`) may wrap at a slightly different byte than the
//!   server. Accept the drift.
//! - The colour-code lookup is the runtime ClassiCube palette
//!   (`Drawer2D.Colors`), so custom palette servers stay in sync.

use classicube_sys::{Convert_CP437ToUnicode, Convert_CodepointToCP437};

use super::is_valid_color_code;

const LIMIT: usize = 64;
const MAX_LINE_LEN: usize = LIMIT + 1;

type IsColor = fn(u8) -> bool;

fn utf8_to_cp437(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| Convert_CodepointToCP437(c as u32))
        .collect()
}

fn cp437_to_utf8(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| char::from_u32(Convert_CP437ToUnicode(b) as u32).unwrap_or('?'))
        .collect()
}

fn last_color(line: &[u8], is_color: IsColor) -> u8 {
    if line.len() < 2 {
        return b'f';
    }
    for i in (0..=line.len() - 2).rev() {
        if line[i] == b'&' && is_color(line[i + 1]) {
            return line[i + 1];
        }
    }
    b'f'
}

fn is_wrapper(line: &[u8], i: usize) -> bool {
    let c = line[i];
    if c == b' ' {
        return true;
    }
    (c == b'-' || c == b'\\') && i > 0 && line[i - 1] != b' '
}

fn starts_with_color(message: &[u8], offset: usize, is_color: IsColor) -> bool {
    message.get(offset).copied() == Some(b'&')
        && message
            .get(offset + 1)
            .map(|&c| is_color(c))
            .unwrap_or(false)
}

fn trim_trailing_invisible(line: &[u8], is_color: IsColor) -> usize {
    let mut len = line.len();
    while len >= 2 {
        let c = line[len - 1];
        if c == b' ' {
            len -= 1;
            continue;
        }
        if line[len - 2] != b'&' {
            break;
        }
        if !is_color(c) {
            break;
        }
        len -= 2;
    }
    len
}

pub fn wordwrap(text: &str) -> Vec<String> {
    wordwrap_impl(text, is_valid_color_code)
}

/// Wrap `text` like the server would, then strip the `> ` each continuation
/// line carries so the bubble shows just the visible content. The leading
/// `&<color>` re-emit (when present) stays intact so wrapped lines keep their
/// color across the break.
pub fn wrap_for_display(text: &str) -> Vec<String> {
    wordwrap(text)
        .into_iter()
        .map(|line| line.strip_prefix("> ").map(str::to_string).unwrap_or(line))
        .collect()
}

/// Same idea as `wrap_for_display`, but accounts for what the server prepends
/// before wrapping: `{nick}: &f`. Without the `&f`, the typing preview's first
/// line would budget two extra CP437 bytes (since the server's color reset
/// eats them) and the wrong `last_col` would carry over to the continuation —
/// producing a spurious `&<nick_color>` re-emit on every wrapped line.
///
/// The `{nick}: ` portion is stripped back off the first output line so the
/// bubble shows just the message; `&f` is left in place so the rendered text
/// stays in the default color the way the server would emit it.
///
/// Char-skip (not byte-strip) because `wordwrap` round-trips through CP437,
/// so non-ASCII chars in a nick may come out as a different UTF-8 byte
/// sequence. Each input char still maps to exactly one CP437 byte and back
/// to one UTF-8 char, so char count is preserved.
pub fn wrap_typing_for_display(text: &str, nick: &str) -> Vec<String> {
    wrap_typing_for_display_impl(text, nick, is_valid_color_code)
}

fn wrap_typing_for_display_impl(text: &str, nick: &str, is_color: IsColor) -> Vec<String> {
    let nick_prefix = format!("{nick}: ");
    let nick_prefix_chars = nick_prefix.chars().count();
    let mut lines = wordwrap_impl(&format!("{nick_prefix}&f{text}"), is_color).into_iter();
    let mut result = Vec::new();
    if let Some(first) = lines.next() {
        result.push(first.chars().skip(nick_prefix_chars).collect());
    }
    for line in lines {
        result.push(line.strip_prefix("> ").map(str::to_string).unwrap_or(line));
    }
    result
}

fn wordwrap_impl(text: &str, is_color: IsColor) -> Vec<String> {
    let message = utf8_to_cp437(text);
    let message_len = message.len();
    if message_len == 0 {
        return Vec::new();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut line = vec![0u8; MAX_LINE_LEN];
    let mut first_line = true;
    let mut last_col = b'f';
    let mut offset = 0usize;

    while offset < message_len {
        let mut length = 0usize;
        if !first_line {
            line[0] = b'>';
            line[1] = b' ';
            length += 2;
            if last_col != b'f' && !starts_with_color(&message, offset, is_color) {
                line[2] = b'&';
                line[3] = last_col;
                length += 2;
            }
        }

        let mut found_start = first_line;
        while length < MAX_LINE_LEN && offset < message_len {
            let c = message[offset];
            offset += 1;
            if c != b' ' || found_start {
                line[length] = c;
                length += 1;
                found_start = true;
            }
        }

        let line_length = LIMIT;
        if length <= line_length {
            let trimmed = trim_trailing_invisible(&line[..length], is_color);
            lines.push(cp437_to_utf8(&line[..trimmed]));
            break;
        }
        first_line = false;

        let lower = LIMIT.saturating_sub(20);
        for i in (lower + 1..line_length).rev() {
            if !is_wrapper(&line, i) {
                continue;
            }
            let new_len = i + 1;
            offset -= length - new_len;
            length = new_len;
            break;
        }

        if length > line_length {
            offset -= length - line_length;
            length = line_length;
        }

        // `length` is at least `LIMIT - 20 + 1 = 45` here: either we entered
        // the wrapper-search branch (which only sets `length = i + 1` for
        // `i >= lower + 1`), or we hit the hard-split clamp to `line_length`.
        if line[length - 1] == b'&' {
            length -= 1;
            offset -= 1;
        }

        last_col = last_color(&line[..length], is_color);
        let trimmed = trim_trailing_invisible(&line[..length], is_color);
        lines.push(cp437_to_utf8(&line[..trimmed]));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ascii_palette(c: u8) -> bool {
        c.is_ascii_hexdigit()
    }

    /// Approximates a ClassiCube server with custom palette colors enabled.
    /// The standard palette is `0-9a-fA-F`; servers can register extras like
    /// `&n`, `&k`, `&m`, `&o` via `/customcolors` (etc.), and `Drawer2D.Colors`
    /// flips them on. Treating every alphanumeric byte as a color matches the
    /// permissive end of that spectrum without enumerating per-server codes.
    fn extended_palette(c: u8) -> bool {
        c.is_ascii_alphanumeric()
    }

    fn wrap(text: &str) -> Vec<String> {
        wordwrap_impl(text, ascii_palette)
    }

    fn wrap_ext(text: &str) -> Vec<String> {
        wordwrap_impl(text, extended_palette)
    }

    #[test]
    fn empty_string() {
        assert_eq!(wrap("").len(), 0);
    }

    #[test]
    fn short_line_no_wrap() {
        assert_eq!(wrap("hello world"), vec!["hello world"]);
    }

    #[test]
    fn exactly_64_chars_no_wrap() {
        let s: String = "a".repeat(64);
        assert_eq!(wrap(&s), vec![s]);
    }

    #[test]
    fn wrap_at_space_boundary() {
        // 60 a's, space, 5 b's = 66 chars total, should wrap at the space.
        let input = format!("{} {}", "a".repeat(60), "b".repeat(5));
        let lines = wrap(&input);
        assert_eq!(lines.len(), 2);
        // First line should end with the space included (per IsWrapper logic).
        assert!(lines[0].starts_with(&"a".repeat(60)));
        assert!(lines[1].starts_with("> "));
        assert!(lines[1].ends_with(&"b".repeat(5)));
    }

    #[test]
    fn wrap_at_hyphen() {
        // long word with a hyphen inside the limit-20..limit window
        let input = format!("{}-{}", "a".repeat(50), "b".repeat(20));
        let lines = wrap(&input);
        assert_eq!(lines.len(), 2);
        // Should split after the hyphen (includes wrapper character).
        assert!(lines[0].ends_with("-"));
    }

    #[test]
    fn hard_split_when_no_wrap_point() {
        // 80 a's straight, no wrappable char in the last 20 of the line.
        let input = "a".repeat(80);
        let lines = wrap(&input);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 64);
        assert!(lines[1].starts_with("> "));
    }

    #[test]
    fn color_reemitted_on_continuation() {
        // 60 chars of &c-colored text, then continuation should re-emit &c.
        let input = format!("&c{}", "a".repeat(80));
        let lines = wrap(&input);
        assert!(lines.len() >= 2);
        assert!(lines[0].starts_with("&c"));
        assert!(lines[1].starts_with("> &c") || lines[1].starts_with("> &"));
    }

    #[test]
    fn trailing_color_code_not_split() {
        // If the split would land mid color code, length and offset back up by 1.
        // Input crafted so position 63 is '&' and pos 64 is a color letter.
        let mut s = "a".repeat(63);
        s.push('&');
        s.push('c');
        s.push_str(&"b".repeat(20));
        let lines = wrap(&s);
        assert!(lines.len() >= 2);
        // First line shouldn't end with a dangling '&'.
        assert!(!lines[0].ends_with("&"));
    }

    #[test]
    fn trim_trailing_invisible_strips_dangling_color() {
        // 62 a's + &c (which would be trimmed as trailing color code).
        let s = format!("{}&c", "a".repeat(62));
        let lines = wrap(&s);
        // Single line case (length=64), color stripped by trim.
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "a".repeat(62));
    }

    #[test]
    fn three_line_wrap() {
        let input = "word ".repeat(40); // 200 chars
        let lines = wrap(&input);
        assert!(lines.len() >= 3);
        for line in &lines[1..] {
            assert!(line.starts_with("> "));
        }
    }

    /// Same 192-char typed message as `server_wrap_with_name_prefix_no_wrap_points`,
    /// but exercises the typing-preview path: prepend `{nick}: &f`, wrap, then
    /// strip the `{nick}: ` off line 1 and `> ` off continuations. The visible
    /// result should match what the server eventually displays — i.e. the
    /// server's lines from that test with the `{nick}: ` removed from line 1
    /// and `> ` removed from lines 2..4.
    #[test]
    fn typing_preview_matches_server_wrap() {
        let typed = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabcaaaaaaaaaaaaaaaaaaaaaadaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let display = wrap_typing_for_display_impl(typed, "&o[&la&o] &6SpiralP", ascii_palette);
        assert_eq!(
            display,
            vec![
                "&faaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabc",
                "aaaaaaaaaaaaaaaaaaaaaadaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ]
        );
    }

    /// Two-line variant: the user types `Message...wrapped` (48 chars), the
    /// server prepends `&o[&la&o] &6SpiralP: &f` (23 CP437 bytes) and wraps
    /// at byte 64, producing
    ///   `&o[&la&o] &6SpiralP: &fMessage..................................`
    ///   `> wrapped`
    /// The typing preview drops the `{nick}: ` from line 1 and `> ` from
    /// line 2, leaving exactly what the bubble should render above the head.
    #[test]
    fn typing_preview_two_line_wrap() {
        let typed = "Message..................................wrapped";
        let display = wrap_typing_for_display_impl(typed, "&o[&la&o] &6SpiralP", ascii_palette);
        assert_eq!(
            display,
            vec!["&fMessage..................................", "wrapped"]
        );
    }

    // Each of the next 5 tests feeds wordwrap a full server-format line with
    // a 32-char nick prefix + `Message` body + dots up to position 64 +
    // `wrapped`. The dots are deliberately inert (no spaces / hyphens / `&`),
    // so the wrap is a hard split at byte 64 — line 1 is the first 64 bytes
    // verbatim, line 2 is `> wrapped` with a re-emitted color iff the last
    // valid `&X` in line 1 wasn't `&f`. These mirror how servers vary their
    // color folding (explicit `&f` reset, redundant codes dropped, etc.).
    //
    // Uses the extended palette so non-standard codes like `&n`, `&k`, `&m`,
    // `&o` count as real color codes (the way they would on a server with
    // custom palette entries registered).

    #[test]
    fn server_wrap_title_colored_nick_colored_message() {
        let input = "[&nTitle&f] &kSpiralP: &fMessage................................wrapped";
        assert_eq!(input.len(), 71);
        let lines = wrap_ext(input);
        assert_eq!(
            lines,
            vec![
                "[&nTitle&f] &kSpiralP: &fMessage................................",
                "> wrapped",
            ]
        );
    }

    #[test]
    fn server_wrap_outer_color_with_trailing_reset() {
        let input = "&o[&nTitle&o] SpiralP&f: Message................................wrapped";
        assert_eq!(input.len(), 71);
        let lines = wrap_ext(input);
        assert_eq!(
            lines,
            vec![
                "&o[&nTitle&o] SpiralP&f: Message................................",
                "> wrapped",
            ]
        );
    }

    #[test]
    fn server_wrap_uncolored_nick_uncolored_message() {
        let input = "[&nTitle&f] SpiralP: Message....................................wrapped";
        assert_eq!(input.len(), 71);
        let lines = wrap_ext(input);
        assert_eq!(
            lines,
            vec![
                "[&nTitle&f] SpiralP: Message....................................",
                "> wrapped",
            ]
        );
    }

    #[test]
    fn server_wrap_uncolored_nick_colored_message() {
        let input = "[&nTitle&f] SpiralP: &mMessage..................................wrapped";
        assert_eq!(input.len(), 71);
        let lines = wrap_ext(input);
        // Last valid `&X` on line 1 is `&m` (just before `Message`), so the
        // continuation re-emits it.
        assert_eq!(
            lines,
            vec![
                "[&nTitle&f] SpiralP: &mMessage..................................",
                "> &mwrapped",
            ]
        );
    }

    #[test]
    fn server_wrap_nick_color_then_trailing_reset() {
        let input = "[&nTitle&f] &aSpiralP&f: Message................................wrapped";
        assert_eq!(input.len(), 71);
        let lines = wrap_ext(input);
        assert_eq!(
            lines,
            vec![
                "[&nTitle&f] &aSpiralP&f: Message................................",
                "> wrapped",
            ]
        );
    }

    #[test]
    fn server_wrap_with_name_prefix_no_wrap_points() {
        // Real session: user types 192 ASCII chars (no spaces or hyphens),
        // server prepends `&o[&la&o] &6SpiralP: &f` (23 CP437 bytes) and
        // wraps at 64 bytes per line. The four resulting lines should match
        // exactly what the server produced. None of the splits land near a
        // color code, so the hex-only test palette suffices.
        let typed = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabcaaaaaaaaaaaaaaaaaaaaaadaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let input = format!("&o[&la&o] &6SpiralP: &f{typed}");
        let lines = wrap(&input);
        assert_eq!(
            lines,
            vec![
                "&o[&la&o] &6SpiralP: &faaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabc",
                "> aaaaaaaaaaaaaaaaaaaaaadaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "> aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "> aaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ]
        );
    }
}
