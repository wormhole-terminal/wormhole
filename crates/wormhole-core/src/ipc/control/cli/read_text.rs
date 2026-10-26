//! `read` and `blocks` as text for a person: the lines of a terminal, and a table of its commands.
//! Everything a terminal printed goes through [`for_terminal`], so what is read cannot drive the
//! terminal it is printed in.

use std::time::Duration;

use crate::ipc::control::printable::for_terminal;
use crate::ipc::control::read::{BlockInfo, Blocks, Read};

/// The lines of a read, one each. Blank lines at the end are left out (a screen has many, and the
/// cursor is in the JSON); `numbers` puts the grid line number before each, right-aligned. A read
/// that had to leave lines out says so first.
pub(super) fn read_text(read: &Read, numbers: bool) -> String {
    let mut out = String::new();
    if read.truncated {
        out.push_str("[earlier lines were left out]\n");
    }
    let shown = read.lines.iter().rposition(|line| !line.trim().is_empty()).map_or(0, |last| last + 1);
    let width = (read.first_line..).take(shown.max(1)).map(|line| line.to_string().len()).max().unwrap_or(1);
    for (at, line) in read.lines[..shown].iter().enumerate() {
        if numbers {
            let number = i64::from(read.first_line) + at as i64;
            out.push_str(&format!("{number:>width$}  "));
        }
        out.push_str(&for_terminal(line));
        out.push('\n');
    }
    out
}

/// The blocks as a table, oldest first, each with its output under it when it has some.
pub(super) fn blocks_text(blocks: &Blocks) -> String {
    if !blocks.integration {
        return "no command blocks: this shell has no integration (see command-blocks in the docs)\n".into();
    }
    if blocks.blocks.is_empty() {
        return "no commands yet\n".into();
    }
    let rows: Vec<[String; 4]> = blocks.blocks.iter().map(row).collect();
    let header = ["ID", "STATE", "EXIT", "TOOK"];
    let widths: Vec<usize> = (0..4)
        .map(|column| {
            rows.iter().map(|row| row[column].len()).chain([header[column].len()]).max().unwrap_or(0)
        })
        .collect();
    let line = |cells: [&str; 4], command: &str| {
        let mut out = String::new();
        for (cell, width) in cells.iter().zip(&widths) {
            out.push_str(&format!("{cell:<width$}  "));
        }
        out.push_str(command);
        out.push('\n');
        out
    };
    let mut out = line(header, "COMMAND");
    for (block, cells) in blocks.blocks.iter().zip(&rows) {
        out.push_str(&line(cells.each_ref().map(String::as_str), &for_terminal(&block.command)));
        if let Some(output) = &block.output {
            if block.truncated {
                out.push_str("    [earlier output left out]\n");
            }
            for text in output.lines() {
                out.push_str(&format!("    {}\n", for_terminal(text)));
            }
        }
    }
    out
}

fn row(block: &BlockInfo) -> [String; 4] {
    [
        block.id.to_string(),
        block.state.clone(),
        block.exit_code.map_or_else(|| "-".into(), |code| code.to_string()),
        block.took_ms.map_or_else(|| "-".into(), |ms| wormhole_term::took_text(Duration::from_millis(ms))),
    ]
}

#[cfg(test)]
mod tests {
    use super::{blocks_text, read_text};
    use crate::ipc::control::read::{BlockInfo, Blocks, CursorInfo, ModesInfo, Read, Size};

    fn read(first_line: i32, lines: &[&str]) -> Read {
        Read {
            terminal: 3,
            title: None,
            screen: "main".into(),
            size: Size { rows: 4, cols: 20 },
            history: 0,
            cursor: CursorInfo { row: 0, col: 0, visible: true, shape: Some("block".into()) },
            modes: ModesInfo {
                app_cursor: false,
                app_keypad: false,
                bracketed_paste: false,
                mouse: "off".into(),
                mouse_encoding: "x10".into(),
                kitty_keyboard: 0,
            },
            first_line,
            lines: lines.iter().map(|line| line.to_string()).collect(),
            styles: None,
            truncated: false,
            revision: 1,
            block: None,
        }
    }

    #[test]
    fn lines_print_one_each_without_the_blank_ones_at_the_end() {
        let text = read_text(&read(0, &["$ ls", "a.rs", "", "b.rs", "", ""]), false);
        assert_eq!(text, "$ ls\na.rs\n\nb.rs\n");
        assert_eq!(read_text(&read(0, &["", ""]), false), "");
    }

    #[test]
    fn a_gutter_of_line_numbers_follows_the_grid_lines() {
        let text = read_text(&read(-2, &["a", "b", "c", "d"]), true);
        assert_eq!(text, "-2  a\n-1  b\n 0  c\n 1  d\n");
        let mut cut = read(10, &["x"]);
        cut.truncated = true;
        assert_eq!(read_text(&cut, true), "[earlier lines were left out]\n10  x\n");
    }

    #[test]
    fn what_a_terminal_printed_cannot_act_on_the_terminal_it_is_printed_in() {
        let text = read_text(&read(0, &["x\u{1b}[2Jy\u{202e}z"]), false);
        assert!(!text.contains(['\u{1b}', '\u{202e}']), "{text:?}");
        assert!(text.contains("\\u{1b}[2J"), "{text}");
    }

    fn block(id: i64, command: &str, state: &str, code: Option<i32>, output: Option<&str>) -> BlockInfo {
        BlockInfo {
            id,
            command: command.into(),
            state: state.into(),
            exit_code: code,
            took_ms: code.map(|_| 3512),
            first_line: 0,
            last_line: 1,
            output: output.map(String::from),
            truncated: false,
        }
    }

    #[test]
    fn blocks_are_a_table_with_their_output_under_them() {
        let blocks = Blocks {
            terminal: 3,
            integration: true,
            blocks: vec![
                block(7, "cargo test", "finished", Some(101), Some("one\ntwo\u{1b}[0m")),
                block(58, "sleep 9", "running", None, None),
            ],
        };
        let text = blocks_text(&blocks);
        assert_eq!(
            text,
            "ID  STATE     EXIT  TOOK  COMMAND\n\
             7   finished  101   3.5s  cargo test\n\
             \x20   one\n\
             \x20   two\\u{1b}[0m\n\
             58  running   -     -     sleep 9\n"
        );
        let none = Blocks { terminal: 3, integration: false, blocks: vec![] };
        assert!(blocks_text(&none).starts_with("no command blocks"));
        let empty = Blocks { terminal: 3, integration: true, blocks: vec![] };
        assert_eq!(blocks_text(&empty), "no commands yet\n");
    }
}
