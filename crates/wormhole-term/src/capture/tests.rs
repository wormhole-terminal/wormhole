use wormhole_vt::{CursorShape, Modes};

use super::{BlocksSpec, CaptureError, CaptureRange, CaptureSpec, Hue, ReadScratch, Style, StyleRun};
use crate::screen::Screen;
use crate::{GridSize, ScreenReader};

fn reader(cols: u16, rows: u16, history: usize) -> ScreenReader {
    ScreenReader::of(Screen::new(GridSize { cols, rows, cell_width: 8.0, cell_height: 16.0 }, history))
}

fn spec(range: CaptureRange) -> CaptureSpec {
    CaptureSpec { range, join: false, styles: false, max_bytes: 256 * 1024 }
}

fn read(reader: &ScreenReader, spec: &CaptureSpec) -> super::Capture {
    reader.capture(spec, &mut ReadScratch::default()).expect("readable")
}

#[test]
fn the_live_screen_comes_with_its_size_cursor_and_modes() {
    let reader = reader(20, 4, 100);
    reader.feed(b"hello\r\nworld\x1b[?1h\x1b[?2004h\x1b[6 q");
    let found = read(&reader, &spec(CaptureRange::Screen));
    assert_eq!(found.lines, ["hello", "world", "", ""]);
    assert_eq!((found.rows, found.cols, found.first_line), (4, 20, 0));
    assert_eq!((found.cursor.row, found.cursor.col, found.cursor.visible), (1, 5, true));
    assert_eq!(found.cursor.shape, Some(CursorShape::Beam));
    assert!(found.modes.contains(Modes::APP_CURSOR | Modes::BRACKETED_PASTE));
    assert!(!found.alt_screen && !found.truncated);
}

#[test]
fn the_alternate_screen_is_what_a_full_screen_program_drew() {
    let reader = reader(20, 3, 100);
    reader.feed(b"shell\r\n\x1b[?1049h\x1b[2;3HVIM\x1b[?25l");
    let found = read(&reader, &spec(CaptureRange::Screen));
    assert!(found.alt_screen && found.modes.contains(Modes::ALT_SCREEN));
    assert_eq!(found.lines, ["", "  VIM", ""]);
    assert_eq!((found.cursor.row, found.cursor.col, found.cursor.visible), (1, 5, false));
    assert_eq!(found.cursor.shape, None, "a hidden cursor has no shape to tell");
    assert_eq!(found.history, 0, "the alternate screen has no history");
}

#[test]
fn history_is_read_by_line_number_and_by_the_last_few() {
    let reader = reader(20, 3, 100);
    for n in 0..10 {
        reader.feed(format!("line {n}\r\n").as_bytes());
    }
    // Ten lines and a cursor line: eight rows have scrolled into the history.
    let all = read(&reader, &spec(CaptureRange::Lines { from: -100, to: 100 }));
    assert_eq!(all.first_line, -(all.history as i32));
    assert_eq!(all.lines.len(), all.history + 3);
    assert_eq!(all.lines[0], "line 0");
    let last = read(&reader, &spec(CaptureRange::Last(2)));
    assert_eq!((last.lines.as_slice(), last.first_line), (&["line 9".to_string(), String::new()][..], 1));
    let some = read(&reader, &spec(CaptureRange::Lines { from: -3, to: -2 }));
    assert_eq!(some.lines, ["line 5", "line 6"].map(String::from));
    let none = read(&reader, &spec(CaptureRange::Lines { from: 7, to: 9 }));
    assert!(none.lines.is_empty());
}

#[test]
fn wrapped_rows_join_into_the_line_the_program_printed() {
    let reader = reader(10, 4, 100);
    reader.feed(b"0123456789abcdef\r\nnext");
    let rows = read(&reader, &spec(CaptureRange::Screen));
    assert_eq!(rows.lines, ["0123456789", "abcdef", "next", ""]);
    let joined = read(&reader, &CaptureSpec { join: true, ..spec(CaptureRange::Screen) });
    assert_eq!(joined.lines, ["0123456789abcdef", "next", ""]);
    assert_eq!(joined.first_line, 0);
}

#[test]
fn colours_and_attributes_come_as_runs_and_the_default_style_is_left_out() {
    let reader = reader(20, 2, 10);
    reader.feed(b"\x1b[1;31mred\x1b[0m plain \x1b[48;2;1;2;3;4mhi");
    let found = read(&reader, &CaptureSpec { styles: true, ..spec(CaptureRange::Screen) });
    let runs = &found.styles.expect("styles")[0];
    let bold_red = Style { fg: Some(Hue::Indexed(1)), bold: true, ..Style::default() };
    assert_eq!(runs[0], StyleRun { col: 0, len: 3, style: bold_red });
    assert_eq!(runs.len(), 2, "{runs:?}");
    assert_eq!((runs[1].col, runs[1].len), (10, 2));
    assert_eq!(runs[1].style.bg, Some(Hue::Rgb(wormhole_vt::Rgb { r: 1, g: 2, b: 3 })));
    assert!(runs[1].style.underline);
}

#[test]
fn hidden_text_wide_characters_and_marks_read_as_they_show() {
    let reader = reader(10, 2, 10);
    reader.feed("ae\u{301}日b\x1b[8msecret\x1b[0m".as_bytes());
    let found = read(&reader, &spec(CaptureRange::Screen));
    assert_eq!(found.lines[0], "ae\u{301}日b", "hidden text is blank, as it is drawn");
}

#[test]
fn text_over_the_byte_cap_keeps_the_newest_lines() {
    let reader = reader(20, 5, 100);
    for n in 0..20 {
        reader.feed(format!("row {n:02}\r\n").as_bytes());
    }
    let found = read(&reader, &CaptureSpec { max_bytes: 30, ..spec(CaptureRange::Last(25)) });
    assert!(found.truncated);
    assert!(found.lines.iter().map(String::len).sum::<usize>() <= 30);
    let newest = found.lines.iter().rev().find(|line| !line.is_empty()).expect("some text");
    assert_eq!(newest, "row 19");
    assert_eq!(found.first_line + found.lines.len() as i32, 5, "the lines end at the screen's bottom");
}

#[test]
fn a_long_range_is_read_in_chunks() {
    let reader = reader(20, 10, 2000);
    for n in 0..1200 {
        reader.feed(format!("line {n}\r\n").as_bytes());
    }
    let found = read(&reader, &spec(CaptureRange::Lines { from: -1000, to: 9 }));
    assert_eq!(found.lines.len(), 1010);
    assert!(!found.truncated);
    let numbers: Vec<usize> =
        found.lines.iter().filter_map(|l| l.strip_prefix("line ")?.parse().ok()).collect();
    assert_eq!(numbers.len(), 1009, "the screen's last row is blank");
    assert!(numbers.windows(2).all(|pair| pair[1] == pair[0] + 1), "consecutive");
}

/// The pins keep a long read's later chunks where its range was, however much is printed in
/// between: the lines come back consecutive, none repeated and none missing.
#[test]
fn output_between_two_chunks_does_not_shift_the_range() {
    let reader = reader(20, 10, 5000);
    for n in 0..1500 {
        reader.feed(format!("line {n}\r\n").as_bytes());
    }
    let mut holds = 0;
    let found = reader
        .capture_between(
            &spec(CaptureRange::Lines { from: -1200, to: 9 }),
            &mut ReadScratch::default(),
            || {
                holds += 1;
                for _ in 0..37 {
                    reader.feed(b"extra\r\n");
                }
            },
        )
        .expect("readable");
    assert!(holds >= 3, "{holds} gaps between chunks");
    assert!(!found.truncated);
    let numbers: Vec<usize> =
        found.lines.iter().filter_map(|l| l.strip_prefix("line ")?.parse().ok()).collect();
    assert_eq!(numbers.len(), 1200 + 10 - 1, "{} lines", found.lines.len());
    assert!(numbers.windows(2).all(|pair| pair[1] == pair[0] + 1), "consecutive");
    assert_eq!(numbers[0], 1500 + 1 - 10 - 1200, "the range began where it was asked to");
}

#[test]
fn history_dropped_while_a_read_goes_on_cuts_it_short() {
    let reader = reader(20, 5, 300);
    for n in 0..300 {
        reader.feed(format!("line {n}\r\n").as_bytes());
    }
    let found = reader
        .capture_between(
            &spec(CaptureRange::Lines { from: -300, to: 4 }),
            &mut ReadScratch::default(),
            || {
                // More than the history keeps, so the rows of the next chunk are gone.
                for _ in 0..20_000 {
                    reader.feed(b"newer\r\n");
                }
            },
        )
        .expect("readable");
    assert!(found.truncated, "{} lines", found.lines.len());
    assert_eq!(found.lines.len(), super::CHUNK, "what was read before the history moved is kept");
    assert_eq!(found.lines[0], "line 0");
}

/// The highest slot a fresh set of pins lands in: pins that were not given back keep their slots,
/// so the number grows with each one left behind.
fn pin_slots(reader: &ScreenReader) -> usize {
    use wormhole_vt::{Column, Line, Point};
    let mut screen = reader.term.lock();
    let pins: Vec<_> = (0..6).map(|_| screen.pin(Point::new(Line(0), Column(0)))).collect();
    let highest = pins
        .iter()
        .map(|pin| {
            let text = format!("{pin:?}");
            let digits: String = text
                .split("index: ")
                .nth(1)
                .unwrap_or("0")
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse::<usize>().unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    for pin in pins {
        screen.unpin(pin);
    }
    highest
}

#[test]
fn nothing_is_left_pinned_after_a_read() {
    let reader = reader(20, 5, 2000);
    for n in 0..800 {
        reader.feed(format!("line {n}\r\n").as_bytes());
    }
    let before = pin_slots(&reader);
    read(&reader, &spec(CaptureRange::Lines { from: -700, to: 4 }));
    assert_eq!(pin_slots(&reader), before);
    // Also when the read is cut short by the history.
    let _ = reader.capture_between(
        &spec(CaptureRange::Lines { from: -700, to: 4 }),
        &mut ReadScratch::default(),
        || {
            for _ in 0..20_000 {
                reader.feed(b"x\r\n");
            }
        },
    );
    assert_eq!(pin_slots(&reader), before);
}

mod blocks {
    use super::*;
    use crate::blocks::BlockState;

    struct Rig {
        reader: ScreenReader,
        prompts: u64,
    }

    impl Rig {
        fn new(rows: u16) -> Self {
            Self { reader: reader(30, rows, 200), prompts: 0 }
        }

        fn feed(&mut self, bytes: &[u8]) {
            let marks = {
                let mut screen = self.reader.term.lock();
                screen.feed(bytes);
                crate::engine::drain_marks(&mut screen, &mut self.prompts)
            };
            for (mark, prompt) in marks {
                self.reader.shared.mark(mark, prompt);
            }
        }
    }

    const SESSION: &[u8] =
        b"\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07a.rs\r\nb.rs\r\n\x1b]133;D;0\x07\
        \x1b]133;A\x07$ \x1b]133;B\x07make\r\n\x1b]133;C\x07error\r\n\x1b]133;D;2\x07\
        \x1b]133;A\x07$ \x1b]133;B\x07";

    fn blocks_spec(last: usize, output: bool) -> BlocksSpec {
        BlocksSpec { last, output, output_bytes: 1000, total_bytes: 2000 }
    }

    #[test]
    fn the_newest_blocks_come_with_their_state_and_output() {
        let mut rig = Rig::new(12);
        rig.feed(SESSION);
        let found = rig.reader.blocks(&blocks_spec(2, true));
        assert!(found.integration);
        let summary: Vec<_> =
            found.blocks.iter().map(|b| (b.id, b.command.as_str(), b.output.as_deref())).collect();
        assert_eq!(summary, [(2, "make", Some("error")), (3, "", None)]);
        assert!(matches!(found.blocks[0].state, BlockState::Finished { code: Some(2), .. }));
        assert_eq!(found.blocks[1].state, BlockState::Prompt);
        let without = rig.reader.blocks(&blocks_spec(10, false));
        assert_eq!(without.blocks.len(), 3);
        assert!(without.blocks.iter().all(|b| b.output.is_none()));
    }

    #[test]
    fn output_is_cut_to_its_newest_bytes_and_the_total_goes_to_the_newest_blocks() {
        let mut rig = Rig::new(12);
        rig.feed(SESSION);
        let tiny = BlocksSpec { last: 3, output: true, output_bytes: 3, total_bytes: 4 };
        let found = rig.reader.blocks(&tiny);
        assert_eq!(found.blocks[1].output.as_deref(), Some("ror"));
        assert!(found.blocks[1].truncated);
        assert_eq!(found.blocks[0].output.as_deref(), Some("s"), "what is left of the total");
    }

    #[test]
    fn a_shell_without_integration_has_no_blocks_and_one_block_is_read_as_lines() {
        let plain = Rig::new(5);
        plain.reader.feed(b"just text\r\n");
        let found = plain.reader.blocks(&blocks_spec(5, true));
        assert!(!found.integration && found.blocks.is_empty());
        let block = plain.reader.capture(&spec(CaptureRange::Block(None)), &mut ReadScratch::default());
        assert_eq!(block.err(), Some(CaptureError::NoIntegration));

        let mut rig = Rig::new(12);
        rig.feed(SESSION);
        let last = read(&rig.reader, &spec(CaptureRange::Block(None)));
        assert_eq!(last.lines, ["$ make", "error"]);
        let block = last.block.expect("the block");
        assert_eq!((block.id, block.command.as_str()), (2, "make"));
        let first = read(&rig.reader, &spec(CaptureRange::Block(Some(1))));
        assert_eq!(first.lines, ["$ ls", "a.rs", "b.rs"]);
        let gone = rig.reader.capture(&spec(CaptureRange::Block(Some(9))), &mut ReadScratch::default());
        assert_eq!(gone.err(), Some(CaptureError::BlockGone));
    }
}
