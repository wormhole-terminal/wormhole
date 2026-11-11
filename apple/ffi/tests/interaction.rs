//! The interaction calls against the real engine and a real PTY: a small shell script prints what the
//! test needs, `cat` stays as the program that reads input (the tty echoes what is written to it).

use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_ffi::{
    CellSide, OpenTarget, PointerAction, PointerButton, PointerEvent, SelectionKind, Terminal,
    TerminalListener, WheelStep,
};

struct Quiet;

impl TerminalListener for Quiet {
    fn wake(&self) {}
}

/// A terminal running `script` and then `cat`, 40 x 10 cells.
fn start(script: &str) -> Arc<Terminal> {
    let args = vec!["-c".to_string(), format!("{script}; exec cat")];
    Terminal::start("/bin/sh".into(), args, 40, 10, 8.0, 16.0, Arc::new(Quiet)).expect("starts")
}

/// Take frames until the screen shows `needle`.
fn wait_for(terminal: &Terminal, needle: &str) -> String {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        terminal.frame();
        let text = terminal.screen_text();
        if text.contains(needle) {
            return text;
        }
        assert!(Instant::now() < until, "never showed {needle:?}; screen:\n{text}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn event(action: PointerAction, button: PointerButton, col: u32, row: u32) -> PointerEvent {
    PointerEvent { button, action, col, row, x: 0, y: 0, shift: false, alt: false, control: false }
}

#[test]
fn a_double_click_selects_a_word_and_the_spans_follow_the_frame() {
    let terminal = start("printf 'hello world\\r\\n'");
    wait_for(&terminal, "hello world");
    terminal.selection_start(SelectionKind::Word, 0, 1, CellSide::Left);
    terminal.selection_update(0, 1, CellSide::Left);
    terminal.selection_finish();
    assert_eq!(terminal.selection_text().as_deref(), Some("hello"));
    terminal.frame();
    let spans = terminal.selection_spans();
    assert_eq!((spans.len(), spans[0].row, spans[0].first, spans[0].last), (1, 0, 0, 4));
    // Typing clears it, as in any terminal.
    terminal.send_key(wormhole_ffi::KeyPress {
        key: "a".into(),
        text: Some("a".into()),
        shift: false,
        alt: false,
        control: false,
        platform: false,
        option_as_meta: false,
    });
    terminal.frame();
    assert!(terminal.selection_spans().is_empty());
}

#[test]
fn a_block_selection_is_a_rectangle() {
    let terminal = start("printf 'abcdef\\r\\nghijkl\\r\\nmnopqr\\r\\n'");
    wait_for(&terminal, "mnopqr");
    terminal.selection_start(SelectionKind::Block, 0, 1, CellSide::Left);
    terminal.selection_update(2, 3, CellSide::Right);
    terminal.frame();
    assert_eq!(terminal.selection_text().as_deref(), Some("bcd\nhij\nnop"));
    assert_eq!(terminal.selection_spans().iter().map(|s| (s.first, s.last)).collect::<Vec<_>>(), [(1, 3); 3]);
}

#[test]
fn a_link_in_the_text_is_found_with_the_cells_it_covers() {
    let terminal = start("printf 'see https://example.com/a now\\r\\n'");
    wait_for(&terminal, "now");
    let hit = terminal.link_at(0, 8).expect("a link under the pointer");
    assert_eq!((hit.uri.as_str(), hit.explicit), ("https://example.com/a", false));
    assert_eq!((hit.spans[0].first, hit.spans[0].last), (4, 24));
    assert!(terminal.link_at(0, 1).is_none());
    assert_eq!(terminal.open_target(hit.uri), OpenTarget::Url { url: "https://example.com/a".into() });
    assert_eq!(terminal.open_target("javascript:alert(1)".into()), OpenTarget::Refused);
}

#[test]
fn an_osc_8_hyperlink_is_explicit() {
    let terminal =
        start("printf '\\033]8;;https://example.org/x\\033\\\\label\\033]8;;\\033\\\\ tail\\r\\n'");
    wait_for(&terminal, "label");
    let hit = terminal.link_at(0, 2).expect("hyperlink");
    assert!(hit.explicit);
    assert_eq!((hit.spans[0].first, hit.spans[0].last), (0, 4));
}

#[test]
fn the_program_gets_the_pointer_only_when_it_asked_and_shift_takes_it_back() {
    let terminal = start("printf '\\033[?1000h\\033[?1006hready\\r\\n'");
    wait_for(&terminal, "ready");
    assert!(terminal.takes_pointer());
    let press = event(PointerAction::Press, PointerButton::Left, 2, 3);
    assert!(terminal.report_pointer(press));
    // The tty echoes the report: ESC shows as ^[ .
    wait_for(&terminal, "^[[<0;3;4M");
    assert!(!terminal.report_pointer(PointerEvent { shift: true, ..press }), "shift: the view selects");
    // Motion in one cell is reported once.
    let motion = event(PointerAction::Motion, PointerButton::Left, 5, 5);
    assert!(terminal.report_pointer(motion) && terminal.report_pointer(motion));
}

#[test]
fn a_program_that_did_not_ask_leaves_the_pointer_to_the_view() {
    let terminal = start("printf 'plain\\r\\n'");
    wait_for(&terminal, "plain");
    assert!(!terminal.takes_pointer());
    assert!(!terminal.report_pointer(event(PointerAction::Press, PointerButton::Left, 0, 0)));
}

#[test]
fn the_wheel_scrolls_history_and_a_trackpad_keeps_its_fractions() {
    let terminal = start("i=0; while [ $i -lt 40 ]; do printf 'line %s\\r\\n' $i; i=$((i+1)); done");
    wait_for(&terminal, "line 39");
    let step = |lines| WheelStep {
        lines,
        columns: 0.0,
        col: 0,
        row: 0,
        x: 0,
        y: 0,
        shift: false,
        alt: false,
        control: false,
    };
    let offset = |terminal: &Terminal| {
        let frame = terminal.frame();
        u32::from_le_bytes(frame[36..40].try_into().expect("header"))
    };
    terminal.scroll_wheel(step(0.5));
    assert_eq!(offset(&terminal), 0, "half a line is not a line");
    terminal.scroll_wheel(step(0.5));
    assert_eq!(offset(&terminal), 1, "two halves are");
    terminal.scroll_wheel(step(3.0));
    assert_eq!(offset(&terminal), 4);
}

#[test]
fn paste_is_bracketed_only_when_the_program_asked() {
    let terminal = start("printf '\\033[?2004hready\\r\\n'");
    wait_for(&terminal, "ready");
    terminal.paste("a\nb".into());
    // ^[ is ESC echoed by the tty; the bracket opens and closes the text, Return for the break.
    let text = wait_for(&terminal, "^[[201~");
    assert!(text.contains("^[[200~a"), "{text}");
}

#[test]
fn dropped_files_are_escaped_with_a_trailing_space() {
    let terminal = start("true");
    assert_eq!(
        terminal.dropped_text(vec!["/tmp/My File.png".into(), "/tmp/b".into()]),
        "/tmp/My\\ File.png /tmp/b "
    );
}

fn press(
    key: &str,
    text: Option<&str>,
    control: bool,
    alt: bool,
    option_as_meta: bool,
) -> wormhole_ffi::KeyPress {
    wormhole_ffi::KeyPress {
        key: key.into(),
        text: text.map(Into::into),
        shift: false,
        alt,
        control,
        platform: false,
        option_as_meta,
    }
}

#[test]
fn keys_follow_the_keyboard_modes_the_program_set() {
    // Kitty keyboard, disambiguate: Control+A is CSI 97;5u, not the byte 1.
    let kitty = start("printf '\\033[>1uready\\r\\n'");
    wait_for(&kitty, "ready");
    assert!(kitty.send_key(press("a", None, true, false, false)));
    wait_for(&kitty, "^[[97;5u");

    // modifyOtherKeys level 2: Control+, has no control byte, so it is CSI 27;5;44~.
    let other = start("printf '\\033[>4;2mready\\r\\n'");
    wait_for(&other, "ready");
    assert!(other.send_key(press(",", None, true, false, false)));
    wait_for(&other, "^[[27;5;44~");

    // Option as Meta: ESC before the key; off, the layout's character goes through.
    let meta = start("printf 'ready\\r\\n'");
    wait_for(&meta, "ready");
    assert!(meta.send_key(press("b", Some("\u{222B}"), false, true, true)));
    wait_for(&meta, "^[b");
}

/// A program that tracks the mouse and has put its cursor on row 0, with the tty raw so `cat -v` shows what arrives.
fn mouse_program() -> Arc<Terminal> {
    let terminal = start("stty raw -echo; printf '\\033[?1000h\\033[?1006hprompt> '; exec cat -v");
    wait_for(&terminal, "prompt>");
    terminal
}

fn drag_on(terminal: &Terminal, row: u32) {
    assert!(terminal.report_pointer(event(PointerAction::Press, PointerButton::Left, 2, row)));
    assert!(terminal.report_pointer(event(PointerAction::Release, PointerButton::Left, 6, row)));
}

#[test]
fn pasting_over_a_selection_the_program_drew_replaces_it() {
    let terminal = mouse_program();
    drag_on(&terminal, 0);
    // The program copied the selection with OSC 52; the clipboard held "before" until then, holds "picked" now.
    terminal.program_copied("picked".into(), Some("before".into()));
    terminal.paste_clipboard(Some("picked".into()));
    // A Backspace first (DEL, `^?`), then what the clipboard held before the program's copy.
    wait_for(&terminal, "^?before");
}

#[test]
fn a_paste_elsewhere_or_after_typing_is_a_plain_paste() {
    let terminal = mouse_program();
    // Selected on a row the cursor is not on: the program would not delete it, so nothing is sent ahead of the paste.
    drag_on(&terminal, 5);
    terminal.program_copied("picked".into(), Some("before".into()));
    terminal.paste_clipboard(Some("picked".into()));
    let screen = wait_for(&terminal, "picked");
    assert!(!screen.contains("^?"), "{screen}");

    // A key the person types ends the program's selection.
    let terminal = mouse_program();
    drag_on(&terminal, 0);
    terminal.program_copied("picked".into(), None);
    terminal.write(b"x".to_vec());
    terminal.paste_clipboard(Some("later".into()));
    let screen = wait_for(&terminal, "xlater");
    assert!(!screen.contains("^?"), "{screen}");
}
