//! Putting a finished block's command back at the prompt, as typed text.
//!
//! The command is read back from the grid, and what is on the grid is what a program drew: text
//! can be drawn so it is not seen (SGR 8, the background's own color, characters that are
//! invisible) and then typed as something else than what the reader saw. So a command is put at
//! the prompt only when every cell of it shows what it holds, and it is only ever typed, never
//! run: the Return is the person's.

use wormhole_vt::{Cell, Color, Flags, Modes};

use super::input;
use super::osc::hides_text;

/// ⌃U: the shell clears the line it has so far.
const KILL_LINE: u8 = 0x15;

/// Why a command is not put at the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Some of it is concealed (SGR 8).
    Concealed,
    /// Some of it is drawn in the color of its own background.
    SameColor,
    /// It holds characters that draw nothing, or blanks that are not spaces.
    Invisible,
    /// It has several lines and the shell does not take a paste as one: each line would run.
    Lines,
}

impl Refusal {
    /// What the notice says, to follow "Not put at the prompt: ".
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::Concealed => "part of the command is concealed",
            Refusal::SameColor => "part of the command is drawn in its background's color",
            Refusal::Invisible => "the command has characters that cannot be seen",
            Refusal::Lines => "it has several lines and this shell does not take a paste",
        }
    }
}

/// The bytes that put `command`, whose cells are `cells`, at the prompt: ⌃U and the text as a paste
/// in `mode`; never a Return. `resolve` turns a cell's colors into what is drawn (the theme and
/// what the program set), so two names for one color count as the same.
pub fn command_to_prompt<C: PartialEq>(
    command: &str,
    cells: &[Cell],
    mode: Modes,
    resolve: impl Fn(Color) -> C,
) -> Result<Vec<u8>, Refusal> {
    for cell in cells {
        // A blank shows nothing in any color.
        if matches!(cell.c, ' ' | '\0' | '\t') || cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        if cell.flags.contains(Flags::HIDDEN) {
            return Err(Refusal::Concealed);
        }
        if resolve(cell.fg) == resolve(cell.bg) {
            return Err(Refusal::SameColor);
        }
    }
    if hides_text(command) {
        return Err(Refusal::Invisible);
    }
    if command.contains('\n') && !mode.contains(Modes::BRACKETED_PASTE) {
        return Err(Refusal::Lines);
    }
    let mut bytes = vec![KILL_LINE];
    bytes.extend(input::paste(command, mode));
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use wormhole_vt::{Cell, Color, Flags, Modes, NamedColor, Rgb};

    use super::{Refusal, command_to_prompt};

    /// A command's cells in the default colors.
    fn typed(text: &str) -> Vec<Cell> {
        text.chars().map(|c| Cell { c, ..Cell::default() }).collect()
    }

    fn put(command: &str, cells: &[Cell], mode: Modes) -> Result<Vec<u8>, Refusal> {
        // Names are their own colors here; a test with a theme gives its own.
        command_to_prompt(command, cells, mode, |color| format!("{color:?}"))
    }

    #[test]
    fn a_command_that_shows_what_it_holds_is_typed_after_clearing_the_line() {
        let bytes = put("ls -l", &typed("ls -l"), Modes::empty()).expect("shown");
        assert_eq!(bytes, b"\x15ls -l");
    }

    #[test]
    fn nothing_that_it_sends_is_a_return() {
        // Without bracketed paste a Return would run the line; a single line has none.
        let bytes = put("make test", &typed("make test"), Modes::empty()).expect("shown");
        assert!(!bytes.contains(&b'\r') && !bytes.contains(&b'\n'), "{bytes:?}");
        // A paste in brackets holds a multi-line command as text, the Returns inside the brackets.
        let bytes =
            put("for x in a\ndo echo $x\ndone", &typed("..."), Modes::BRACKETED_PASTE).expect("shown");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("\x15\x1b[200~") && text.ends_with("\x1b[201~"), "{text:?}");
        let inside = text.trim_start_matches("\x15\x1b[200~").trim_end_matches("\x1b[201~");
        assert_eq!(inside.matches('\r').count(), 2, "only the command's own line breaks, inside the paste");
        // Outside brackets every line break would be a Return, so a command with one is refused.
        assert_eq!(put("a\nb", &typed("a b"), Modes::empty()), Err(Refusal::Lines));
    }

    #[test]
    fn concealed_text_is_refused_wherever_it_sits() {
        let mut cells = typed("ls rm -rf ~");
        cells[3].flags |= Flags::HIDDEN;
        assert_eq!(put("ls rm -rf ~", &cells, Modes::empty()), Err(Refusal::Concealed));
        // A concealed blank hides nothing.
        let mut cells = typed("ls  x");
        cells[2].flags |= Flags::HIDDEN;
        assert!(put("ls  x", &cells, Modes::empty()).is_ok());
    }

    #[test]
    fn text_in_its_backgrounds_color_is_refused_by_what_is_drawn_not_by_the_names() {
        // Black text (palette entry 0) on the default background: two names for one color in a
        // theme whose background is black, two colors in one whose background is not.
        let mut cells = typed("ls x");
        cells[3].fg = Color::Indexed(0);
        let black_background = |color: Color| match color {
            Color::Named(NamedColor::Background) | Color::Indexed(0) => 0,
            _ => 1,
        };
        assert_eq!(
            command_to_prompt("ls x", &cells, Modes::empty(), black_background),
            Err(Refusal::SameColor)
        );
        let grey_background = |color: Color| match color {
            Color::Named(NamedColor::Background) => 2,
            Color::Indexed(0) => 0,
            _ => 1,
        };
        assert!(command_to_prompt("ls x", &cells, Modes::empty(), grey_background).is_ok());
        // The same true color on both sides.
        let mut cells = typed("ls x");
        cells[3].fg = Color::Rgb(Rgb { r: 9, g: 9, b: 9 });
        cells[3].bg = Color::Rgb(Rgb { r: 9, g: 9, b: 9 });
        assert_eq!(put("ls x", &cells, Modes::empty()), Err(Refusal::SameColor));
    }

    #[test]
    fn characters_that_draw_nothing_are_refused() {
        for command in ["ls\u{200B} x", "rm\u{202E}fr -", "echo a\u{E0041}", "a\u{00A0}b"] {
            assert_eq!(put(command, &typed(command), Modes::empty()), Err(Refusal::Invisible), "{command:?}");
        }
    }
}
