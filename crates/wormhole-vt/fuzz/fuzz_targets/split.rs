#![no_main]

//! Where a read is cut must not matter: a terminal fed a stream whole and one fed the same stream
//! in pieces end up with the same screen, history, cursor, title and replies.

use libfuzzer_sys::fuzz_target;
use wormhole_vt::{Line, Terminal};

fn state(term: &mut Terminal) -> String {
    let events: Vec<_> = term.events().collect();
    let replies = term.take_replies();
    let history: Vec<String> =
        (1..=term.history_len() as i32).map(|n| term.row_text(Line(-n), false)).collect();
    format!(
        "{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{history:?}\n{events:?}",
        term.screen_text(),
        term.cursor_point(),
        term.cursor(),
        term.title(),
        replies,
        term.mode(),
    )
}

fuzz_target!(|data: &[u8]| {
    // Four bytes: the size and how many cuts; then the cuts; then the stream.
    let [cols, rows, history, count, rest @ ..] = data else { return };
    let (cols, rows) = (1 + usize::from(*cols) % 60, 1 + usize::from(*rows) % 20);
    let history = [0, 3, 40, 500][usize::from(*history & 3)];
    let count = (usize::from(*count) % 12).min(rest.len());
    let (cuts, stream) = rest.split_at(count);
    let mut at: Vec<usize> = cuts.iter().map(|&c| c as usize % (stream.len() + 1)).collect();
    at.sort_unstable();

    let mut whole = Terminal::builder().size(cols, rows).scrollback(history).build();
    whole.feed(stream);

    let mut split = Terminal::builder().size(cols, rows).scrollback(history).build();
    let mut from = 0;
    for &cut in &at {
        split.feed(&stream[from..cut]);
        from = cut;
    }
    split.feed(&stream[from..]);

    assert_eq!(state(&mut whole), state(&mut split), "cut at {at:?}");
});
