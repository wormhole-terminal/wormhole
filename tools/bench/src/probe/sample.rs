//! The sample page: what can't be asked is shown, for eyes and screenshots. Underline styles,
//! hyperlinks and text attributes either draw or they don't; glyph runs on a ruler show whether a
//! terminal keeps every cell where the grid says it is.

const ESC: &str = "\x1b";

fn heading(s: &mut String, text: &str) {
    s.push_str(&format!("\n{ESC}[1m{text}{ESC}[22m\n"));
}

/// Cells of a ruler `n` wide: tens digit marks and units, `0123456789` repeated, then a `|`.
fn ruler(n: usize) -> String {
    let digits: String = (0..n).map(|i| char::from(b'0' + (i % 10) as u8)).collect();
    format!("{digits}|")
}

/// A row of `glyph` repeated `n` times, closed with the `|` the ruler ends in: if the terminal draws
/// the glyph at another width than one cell, the bar is not under the ruler's.
fn run(glyphs: &str, n: usize) -> String {
    format!("{}|", glyphs.chars().cycle().take(n).collect::<String>())
}

/// Items with their expected cell widths, each followed by `|`; the line above marks where every
/// `|` belongs.
fn grid(items: &[(&str, usize)]) -> (String, String) {
    let mut marks = String::new();
    let mut row = String::new();
    for (glyph, w) in items {
        marks.push_str(&"-".repeat(*w));
        marks.push('|');
        row.push_str(glyph);
        row.push('|');
    }
    (marks, row)
}

pub fn page() -> String {
    let mut s = String::new();
    s.push_str(&format!("{ESC}[0m"));
    heading(&mut s, "ttyprobe sample page  (look at it; none of this can be asked)");

    heading(&mut s, "Text attributes");
    s.push_str(&format!(
        "{ESC}[1mbold{ESC}[22m  {ESC}[2mdim{ESC}[22m  {ESC}[3mitalic{ESC}[23m  {ESC}[9mstrikethrough{ESC}[29m  \
         {ESC}[53moverline{ESC}[55m  {ESC}[5mblink{ESC}[25m  {ESC}[7mreverse{ESC}[27m  {ESC}[1;3mbold italic{ESC}[22;23m\n"
    ));

    heading(&mut s, "Underlines");
    for (n, name) in [(1, "single"), (2, "double"), (3, "curly"), (4, "dotted"), (5, "dashed")] {
        s.push_str(&format!("{ESC}[4:{n}m{name}{ESC}[24m  "));
    }
    s.push('\n');
    s.push_str(&format!(
        "{ESC}[4:1;58:2::255:90:90mred single{ESC}[24;59m  {ESC}[4:3;58:2::90:200:255mblue curly{ESC}[24;59m  \
         {ESC}[4:2;58:2::120:230:140mgreen double{ESC}[24;59m\n"
    ));

    heading(&mut s, "Hyperlink (OSC 8)");
    s.push_str(&format!(
        "{ESC}]8;;https://github.com/wormhole-terminal/ttystorm{ESC}\\ttystorm on GitHub{ESC}]8;;{ESC}\\  \
         (underlined or clickable if supported)\n"
    ));

    heading(&mut s, "Box drawing, blocks and braille, one cell each: the | must sit under the ruler's");
    let width = 40;
    s.push_str(&format!("{}\n", ruler(width)));
    s.push_str(&format!("{}\n", run("─", width)));
    s.push_str(&format!("{}\n", run("┌┬┐├┼┤└┴┘", width)));
    s.push_str(&format!("{}\n", run("═║╔╦╗╠╬╣╚╩╝", width)));
    s.push_str(&format!("{}\n", run("▁▂▃▄▅▆▇█", width)));
    s.push_str(&format!("{}\n", run("░▒▓█▀▄▌▐", width)));
    s.push_str(&format!("{}\n", run("⠁⠃⠇⡇⣇⣧⣷⣿", width)));

    heading(&mut s, "Powerline and Nerd Font symbols (blank boxes mean a patched font is missing)");
    s.push_str(&format!(
        "{ESC}[48;2;56;189;248;38;2;14;16;26m \u{e0a0} main {ESC}[38;2;56;189;248;48;2;129;140;248m\u{e0b0}\
         {ESC}[38;2;14;16;26m ~/ttystorm {ESC}[38;2;129;140;248;48;2;192;132;252m\u{e0b0}\
         {ESC}[38;2;14;16;26m ok {ESC}[0m{ESC}[38;2;192;132;252m\u{e0b0}{ESC}[0m   \
         \u{e0b1} \u{e0b3} \u{e0b2} \u{e0b4} \u{e0b6} \u{e0a1} \u{e0a2}\n"
    ));

    heading(&mut s, "Wide characters, each followed by | : the bars belong under the dashes' bars");
    let (marks, row) = grid(&[
        ("a", 1),
        ("中", 2),
        ("文", 2),
        ("ｱ", 1),
        ("한", 2),
        ("😀", 2),
        ("🎉", 2),
        ("👍🏽", 2),
        ("🇯🇵", 2),
        ("👨‍👩‍👧", 2),
        ("❤️", 2),
        ("⚠️", 2),
        ("⚠", 1),
        ("1️⃣", 2),
        ("é", 1),
        ("→", 1),
    ]);
    s.push_str(&format!("{marks}\n{row}\n"));
    let (marks, row) =
        grid(&[("中", 2), ("文", 2), ("字", 2), ("日", 2), ("本", 2), ("語", 2), ("한", 2), ("글", 2)]);
    s.push_str(&format!("{marks}\n{row}\n"));
    let (marks, row) =
        grid(&[("😀", 2), ("😃", 2), ("😄", 2), ("😁", 2), ("🚀", 2), ("🔥", 2), ("✅", 2), ("❌", 2)]);
    s.push_str(&format!("{marks}\n{row}\n"));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rulers_and_runs_line_up() {
        assert_eq!(ruler(12), "012345678901|");
        assert_eq!(run("ab", 5), "ababa|");
        assert_eq!(ruler(40).chars().count(), run("─", 40).chars().count());
    }

    #[test]
    fn grid_marks_sit_over_the_bars() {
        let (marks, row) = grid(&[("a", 1), ("中", 2)]);
        assert_eq!(marks, "-|--|");
        assert_eq!(row, "a|中|");
    }

    #[test]
    fn the_page_asks_for_each_style() {
        let p = page();
        for needle in
            ["4:1m", "4:2m", "4:3m", "4:4m", "4:5m", "58:2::", "\x1b[3m", "\x1b[53m", "]8;;https://"]
        {
            assert!(p.contains(needle), "{needle:?}");
        }
        assert!(p.ends_with('\n'));
    }
}
