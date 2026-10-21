//! The history that leaves the engine, handed to the host: `Event::HistoryDropped`.

use std::mem::size_of;

use crate::grid::{DropReason, DroppedHistory, HistoryLine};
use crate::terminal::{Event, Limits, Terminal};
use crate::types::{Color, Flags, Line, NamedColor, Rgb};

/// A terminal that archives, 20 columns by 5 rows: narrow, so that lines wrap and a few thousand
/// of them are enough to make the grid drop pages (it keeps about 2,000 lines more than asked).
fn archiving(scrollback: usize) -> Terminal {
    Terminal::builder().size(20, 5).scrollback(scrollback).archive_history(true).build()
}

/// The history batches among the events since the last call.
fn dropped(term: &mut Terminal) -> Vec<DroppedHistory> {
    term.events()
        .filter_map(|event| match event {
            Event::HistoryDropped(batch) => Some(batch),
            _ => None,
        })
        .collect()
}

/// The text of an archived row, one character per cell (a wide character's second cell is empty).
fn row_string(line: &HistoryLine) -> String {
    line.cells
        .iter()
        .filter(|cell| !cell.flags.contains(Flags::WIDE_CHAR_SPACER))
        .map(|cell| cell.c)
        .collect()
}

/// The rows the terminal still has, history first: their text and whether they wrap.
fn kept_rows(term: &Terminal) -> Vec<(String, bool)> {
    let history = term.history_len() as i32;
    (-history..term.rows() as i32)
        .map(|line| (term.row_text(Line(line), false), term.is_wrapped(Line(line))))
        .collect()
}

/// The lines of text the rows make, wrapped rows joined, trailing blanks and empty lines gone.
fn logical(rows: impl IntoIterator<Item = (String, bool)>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for (text, wrapped) in rows {
        current.push_str(&text);
        if !wrapped {
            let line = current.trim_end().to_string();
            if !line.is_empty() {
                lines.push(line);
            }
            current.clear();
        }
    }
    lines
}

fn archived_rows(batches: &[DroppedHistory]) -> Vec<(String, bool)> {
    batches.iter().flat_map(|b| &b.lines).map(|line| (row_string(line), line.wrapped)).collect()
}

/// Line `i` of a stream of numbered lines, some longer than the screen is wide.
fn numbered(i: usize) -> String {
    format!("{i:05} {}", "x".repeat(i * 7 % 45))
}

#[test]
fn nothing_is_handed_over_unless_asked() {
    let mut term = Terminal::builder().size(20, 5).scrollback(20).build();
    assert!(!term.archive_history());
    for i in 0..6000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    assert!(dropped(&mut term).is_empty());
}

/// Everything ever printed is in the archive, the history or on the screen, once and in order.
#[test]
fn archived_plus_kept_is_everything_that_was_printed() {
    let mut term = archiving(20);
    let mut text = String::new();
    let mut archive = Vec::new();
    let mut all = Vec::new();
    for i in 0..9000 {
        let line = numbered(i);
        text.push_str(&line);
        text.push_str("\r\n");
        all.push(line.trim_end().to_string());
        // The host feeds what it has read and takes the events between reads, with odd cuts.
        if text.len() > 777 {
            term.feed(text.as_bytes());
            text.clear();
            archive.extend(dropped(&mut term));
        }
    }
    term.feed(text.as_bytes());
    archive.extend(dropped(&mut term));

    assert!(archive.iter().all(|b| b.reason == DropReason::Scrolled && b.skipped == 0));
    let archived = archived_rows(&archive);
    assert!(archived.len() > 5000, "{} lines archived", archived.len());
    let rows = archived.into_iter().chain(kept_rows(&term));
    assert_eq!(logical(rows), all);
}

/// The same, with the host not taking the events until the end (the backlog is big enough).
#[test]
fn the_events_can_wait_for_the_end() {
    let mut term = archiving(20);
    let mut all = Vec::new();
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
        all.push(numbered(i).trim_end().to_string());
    }
    let archive = dropped(&mut term);
    assert!(archive.iter().all(|b| b.skipped == 0));
    let rows = archived_rows(&archive).into_iter().chain(kept_rows(&term));
    assert_eq!(logical(rows), all);
}

#[test]
fn a_wrapped_line_comes_out_in_rows_that_say_so() {
    let mut term = archiving(20);
    let long = "0123456789".repeat(5);
    for _ in 0..3000 {
        term.feed(format!("{long}\r\nshort\r\n").as_bytes());
    }
    let batches = dropped(&mut term);
    let rows = archived_rows(&batches);
    // 50 characters in 20 columns: three rows, two of them wrapped, then the short line's row.
    let at = rows.iter().position(|(text, _)| text.starts_with("0123456789")).unwrap_or(0);
    let pattern: Vec<(String, bool)> = rows[at..at + 4].to_vec();
    assert_eq!(
        pattern.iter().map(|(_, wrapped)| *wrapped).collect::<Vec<_>>(),
        [true, true, false, false],
        "{pattern:?}"
    );
    assert_eq!(logical(pattern), [long, "short".to_string()]);
    let lines = &batches[0].lines;
    assert!(lines[at].cells[19].flags.contains(Flags::WRAPLINE), "the row's last cell carries the wrap");
    assert!(!lines[at + 2].cells[19].flags.contains(Flags::WRAPLINE));
}

#[test]
fn colours_links_and_marks_survive() {
    let mut term = archiving(20);
    term.feed(
        "\x1b[1;31mred\x1b[0m \x1b]8;id=a;https://example.com/x\x07link\x1b]8;;\x07 e\u{301}\x1b[4;58;2;1;2;3mu\x1b[0m\r\n"
            .as_bytes(),
    );
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    let batches = dropped(&mut term);
    let first = &batches[0].lines[0];
    assert_eq!(row_string(first).trim_end(), "red link eu");
    let red = Color::Named(NamedColor::Red);
    assert_eq!(first.cells[0].fg, red);
    assert!(first.cells[0].flags.contains(Flags::BOLD));
    assert_eq!(first.cells[4].c, 'l');
    let link = first.extras.hyperlink(4).map(|l| (&*l.id, &*l.uri));
    assert_eq!(link, Some(("a", "https://example.com/x")));
    assert_eq!(first.extras.hyperlink(7).map(|l| &*l.uri), Some("https://example.com/x"));
    assert!(first.extras.hyperlink(8).is_none());
    assert_eq!(first.extras.zerowidth(9), ['\u{301}']);
    assert_eq!(first.extras.underline_color(10), Some(Color::Rgb(Rgb { r: 1, g: 2, b: 3 })));
    assert!(!first.wrapped);
}

#[test]
fn a_line_scrolled_out_keeps_its_shell_mark() {
    let mut term = archiving(20);
    term.feed(b"\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n");
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    let batches = dropped(&mut term);
    let mark = batches[0].lines[0].mark;
    assert!(mark.is_some_and(|m| m.block == 1 && m.prompt && m.command), "{mark:?}");
}

#[test]
fn clearing_the_history_hands_it_over_with_its_reason() {
    for how in ["ed3", "clear_history", "ris"] {
        let mut term = archiving(20000);
        for i in 0..1000 {
            term.feed(format!("{}\r\n", numbered(i)).as_bytes());
        }
        let history = term.history_len();
        assert!(history > 900 && dropped(&mut term).is_empty(), "{how}: nothing dropped yet");
        match how {
            "ed3" => term.feed(b"\x1b[3J"),
            "clear_history" => term.clear_history(),
            _ => term.feed(b"\x1bc"),
        }
        let batches = dropped(&mut term);
        assert_eq!(batches.len(), 1, "{how}");
        assert_eq!(batches[0].reason, DropReason::Cleared, "{how}");
        assert_eq!(batches[0].lines.len(), history, "{how}");
        assert_eq!(term.history_len(), 0);
        let lines = logical(archived_rows(&batches));
        assert!(lines[0].starts_with("00000"), "{how}: oldest first");
        assert!(lines.windows(2).all(|w| w[0][..5] < w[1][..5]), "{how}: in order");
    }
}

#[test]
fn lowering_the_scrollback_hands_over_what_is_cut() {
    let mut term = archiving(100_000);
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    assert!(dropped(&mut term).is_empty());
    let before = term.history_len();
    term.set_scrollback_lines(10);
    let batches = dropped(&mut term);
    assert!(batches.iter().all(|b| b.reason == DropReason::Limit && b.skipped == 0));
    let count: usize = batches.iter().map(|b| b.lines.len()).sum();
    assert_eq!(count, before - term.history_len());
    assert!(count > 3000, "{count}");
    assert!(archived_rows(&batches)[0].0.starts_with("00000"));
}

#[test]
fn a_resize_that_cuts_the_history_says_limit() {
    let mut term = archiving(3000);
    for i in 0..6000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    let dropped_before: usize = dropped(&mut term).iter().map(|b| b.lines.len()).sum();
    let history = term.history_len();
    // Four times the columns: a row costs four times as much, so a fourth as many fit.
    term.resize(80, 5);
    let batches = dropped(&mut term);
    assert!(!batches.is_empty() && batches.iter().all(|b| b.reason == DropReason::Limit), "{batches:?}");
    let cut: usize = batches.iter().map(|b| b.lines.len()).sum();
    assert_eq!(cut, history - term.history_len(), "all that went was handed over (was {dropped_before})");
    // The history is reflowed to the new width before the limit cuts it: the rows that left have it.
    assert_eq!(batches[0].lines[0].cells.len(), 80);
}

#[test]
fn reasons_arrive_in_the_order_they_happened() {
    let mut term = archiving(20);
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    term.feed(b"\x1b[3J");
    let reasons: Vec<_> = dropped(&mut term).iter().map(|b| b.reason).collect();
    assert_eq!(reasons, [DropReason::Scrolled, DropReason::Cleared]);
}

#[test]
fn what_the_host_does_not_take_is_counted_not_kept() {
    let mut limits = Limits::default();
    let cost = 20 * size_of::<crate::types::Cell>() + size_of::<HistoryLine>();
    limits.archive_backlog = 300 * cost;
    let mut term =
        Terminal::builder().size(20, 5).scrollback(20).limits(limits).archive_history(true).build();
    for i in 0..6000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    let scrolled = 6000 - 4;
    let batches = dropped(&mut term);
    let kept: usize = batches.iter().map(|b| b.lines.len()).sum();
    let skipped: usize = batches.iter().map(|b| b.skipped).sum();
    assert_eq!(kept, 300);
    assert_eq!(kept + skipped, scrolled - term.history_len(), "every line is either kept or counted");
    assert_eq!(batches.last().map(|b| b.skipped), Some(skipped), "the lost ones are the newest");
    let texts = archived_rows(&batches);
    assert!(texts[0].0.starts_with("00000") && texts[299].0.starts_with("00299"), "the oldest, in order");

    // The host caught up: the lines that leave next are kept again, after the ones that were lost.
    for i in 6000..9000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    let batches = dropped(&mut term);
    let kept: usize = batches.iter().map(|b| b.lines.len()).sum();
    assert_eq!(kept, 300, "a fresh backlog");
    assert!(batches[0].lines.len() == 300 && batches[0].skipped > 0);
}

#[test]
fn a_backlog_set_later_applies_to_the_archive_that_is_running() {
    let cost = 20 * size_of::<crate::types::Cell>() + size_of::<HistoryLine>();
    let mut term = archiving(20);
    term.set_limits(Limits { archive_backlog: 300 * cost, ..Limits::default() });
    for i in 0..6000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    let kept: usize = dropped(&mut term).iter().map(|b| b.lines.len()).sum();
    assert_eq!(kept, 300);

    // Limits do not switch an archive on that was not asked for.
    let mut term = Terminal::builder().size(20, 5).scrollback(20).build();
    term.set_limits(Limits { archive_backlog: 300 * cost, ..Limits::default() });
    for i in 0..6000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    assert!(dropped(&mut term).is_empty());
}

#[test]
fn a_skip_stays_in_the_batch_of_its_reason() {
    let limits = Limits { archive_backlog: 1, ..Limits::default() }; // less than a line
    let mut term =
        Terminal::builder().size(20, 5).scrollback(20000).limits(limits).archive_history(true).build();
    for i in 0..1000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    term.feed(b"\x1b[3J");
    let batches = dropped(&mut term);
    assert_eq!(batches.len(), 1);
    assert_eq!((batches[0].reason, batches[0].lines.len()), (DropReason::Cleared, 0));
    assert!(batches[0].skipped > 900);
}

#[test]
fn the_archive_is_not_held_to_the_event_queues_cap() {
    let limits = Limits { events: 1, ..Limits::default() };
    let mut term =
        Terminal::builder().size(20, 5).scrollback(20).limits(limits).archive_history(true).build();
    for i in 0..9000 {
        term.feed(format!("{}\r\n\x07\x1b]9;n{i}\x07", numbered(i)).as_bytes());
    }
    let batches = dropped(&mut term);
    assert!(batches.iter().map(|b| b.lines.len()).sum::<usize>() > 5000);
}

#[test]
fn the_alternate_screen_has_no_history_to_hand_over() {
    let mut term = archiving(20);
    term.feed(b"before\r\n\x1b[?1049h");
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    assert!(dropped(&mut term).is_empty());
    term.feed(b"\x1b[?1049l");
    assert!(dropped(&mut term).is_empty());
    assert_eq!(term.history_len(), 0);
}

#[test]
fn the_main_screens_history_is_archived_while_the_alternate_screen_shows() {
    let mut term = archiving(100_000);
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    term.feed(b"\x1b[?1049h");
    term.set_scrollback_lines(10);
    let batches = dropped(&mut term);
    assert!(batches.iter().all(|b| b.reason == DropReason::Limit) && !batches.is_empty());
    // The switch took the archive along with the main screen.
    term.feed(b"\x1b[?1049l\x1b[3J");
    assert_eq!(dropped(&mut term)[0].reason, DropReason::Cleared);
}

#[test]
fn switching_it_off_hands_over_what_waits_and_stops() {
    let mut term = archiving(20);
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    term.set_archive_history(false);
    assert!(!term.archive_history());
    assert!(dropped(&mut term).iter().map(|b| b.lines.len()).sum::<usize>() > 5000);
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    assert!(dropped(&mut term).is_empty());
}

#[test]
fn switching_it_on_later_hands_over_only_what_leaves_afterwards() {
    let mut term = Terminal::builder().size(20, 5).scrollback(20).build();
    for i in 0..9000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    term.set_archive_history(true);
    assert!(term.archive_history());
    assert!(dropped(&mut term).is_empty());
    for i in 9000..18000 {
        term.feed(format!("{}\r\n", numbered(i)).as_bytes());
    }
    let batches = dropped(&mut term);
    // The first line to leave was in the history when it was switched on, a few hundred lines back
    // (the first row may be the rest of a line that started before).
    let lines = logical(archived_rows(&batches));
    assert!(lines[1][..5].parse::<usize>().is_ok_and(|n| (7000..9000).contains(&n)), "{}", lines[1]);
}

#[test]
fn history_leaving_a_terminal_without_any_scrollback_is_archived_too() {
    // The grid keeps a few hundred lines even for none; what leaves is still handed over.
    let mut term = archiving(0);
    for i in 0..9000 {
        term.feed(format!("{i:05}\r\n").as_bytes());
    }
    let mut rows = archived_rows(&dropped(&mut term));
    rows.extend(kept_rows(&term));
    let all: Vec<String> = (0..9000).map(|i| format!("{i:05}")).collect();
    assert_eq!(logical(rows), all);
}
