//! What a program's sequences ask of the host.

use super::Rig;
use crate::grid::RowMark;
use crate::terminal::{Event, Progress, ProgressState, PromptKind, ShellMark, Terminator};
use crate::types::Line;

#[test]
fn titles_are_set_reset_and_stacked() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]0;one\x07");
    assert_eq!(rig.term.title(), Some("one"));
    assert_eq!(rig.events(), [Event::Title]);
    rig.feed("\x1b[22;0t\x1b]2;two\x1b\\\x1b[23;0t");
    assert_eq!(rig.term.title(), Some("one"));
    assert_eq!(rig.events(), [Event::Title]);
    rig.feed("\x1b]2;\x07");
    assert_eq!(rig.term.title(), None, "an empty title is no title");
    rig.feed("\x1b]1;icon\x07");
    assert_eq!(rig.events(), [Event::Title, Event::IconName("icon".into())]);
    // Popping with nothing on the stack does nothing.
    rig.feed("\x1b[23;0t");
    assert!(rig.events().is_empty());
}

#[test]
fn a_long_title_is_cut_at_a_character() {
    let mut rig = Rig::new(20, 5);
    rig.feed(&format!("\x1b]0;{}\x07", "é".repeat(3000)));
    let title = rig.term.title().map(str::len).unwrap_or(0);
    assert!(title <= 4096 && title > 4000, "{title} bytes");
}

#[test]
fn the_working_folder_is_kept_and_announced() {
    let mut rig = Rig::new(20, 5);
    assert_eq!(rig.term.cwd(), None);
    rig.feed("\x1b]7;file://host/work/app%20x\x07\x1b]7;file://host/work/lib\x1b\\");
    assert_eq!(rig.term.cwd(), Some("file://host/work/lib"));
    // Every report is kept, though the two make one event; a repeat is a report too.
    assert_eq!(rig.events(), [Event::Cwd], "two in a row are one event");
    assert_eq!(rig.term.cwd_reports(), ["file://host/work/app%20x", "file://host/work/lib"]);
    rig.term.clear_cwd_reports();
    assert!(rig.term.cwd_reports().is_empty());
    // ConEmu's form.
    rig.feed("\x1b]9;9;\"/tmp/x\"\x07");
    assert_eq!(rig.term.cwd(), Some("/tmp/x"));
}

#[test]
fn a_bell_is_one_event_however_many() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x07\x07\x07");
    assert_eq!(rig.events(), [Event::Bell]);
    rig.feed("\x07a\x07");
    assert_eq!(rig.events(), [Event::Bell], "a bell, a letter and a bell: still one before the host looks");
    rig.feed("\x07\x1b]0;t\x07\x07");
    assert_eq!(rig.events(), [Event::Bell, Event::Title, Event::Bell]);
}

#[test]
fn notifications_and_progress() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]9;build done\x07\x1b]777;notify;Title;the body; with a semicolon\x07");
    rig.feed("\x1b]99;i=1:d=0;hel\x1b\\\x1b]9;4;1;40\x07\x1b]9;4;3\x07\x1b]9;4;0\x07\x1b]9;4;2;250\x07\x1b]9;4;9\x07");
    assert_eq!(
        rig.events(),
        [
            Event::Notification { title: String::new(), body: "build done".into() },
            Event::Notification { title: "Title".into(), body: "the body; with a semicolon".into() },
            Event::KittyNotification { metadata: "i=1:d=0".into(), payload: "hel".into() },
            Event::Progress(Some(Progress { state: ProgressState::Normal, percent: 40 })),
            Event::Progress(Some(Progress { state: ProgressState::Indeterminate, percent: 0 })),
            Event::Progress(None),
            Event::Progress(Some(Progress { state: ProgressState::Error, percent: 100 })),
        ]
    );
}

#[test]
fn the_clipboard_is_stored_and_asked_for() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]52;c;aGVsbG8=\x07\x1b]52;;aGk\x1b\\\x1b]52;p;?\x07\x1b]52;c;!not base64\x07");
    assert_eq!(
        rig.events(),
        [
            Event::ClipboardStore { selection: "c".into(), text: "hello".into() },
            Event::ClipboardStore { selection: "s0".into(), text: "hi".into() },
            Event::ClipboardLoad { selection: "p".into(), terminator: Terminator::Bel, replies_before: 0 },
        ]
    );
    rig.term.clipboard_reply("p", "text", Terminator::Bel);
    assert_eq!(rig.replies(), "\x1b]52;p;dGV4dA==\x07");
    rig.term.clipboard_reply("c", "", Terminator::St);
    assert_eq!(rig.replies(), "\x1b]52;c;\x1b\\");
}

#[test]
fn a_clipboard_request_names_only_selections_that_exist() {
    let mut rig = Rig::new(20, 5);
    // What a program puts where the selection goes is repeated in the reply, into its own input.
    rig.feed("\x1b]52;c\x7f\x7frm -rf ~ #\u{85};?\x07\x1b]52;xyz;?\x07");
    rig.feed(&format!("\x1b]52;{};?\x07", "a".repeat(100_000)));
    rig.feed(&format!("\x1b]52;{};?\x07", "p".repeat(100)));
    let selections: Vec<String> = rig
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::ClipboardLoad { selection, .. } => Some(selection),
            _ => None,
        })
        .collect();
    assert_eq!(selections, ["c", "s0", "s0", "pppppppppppp"]);
}

#[test]
fn a_clipboard_request_says_how_many_reply_bytes_came_before_it() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[6n\x1b]52;c;?\x07\x1b[5n");
    assert_eq!(
        rig.events(),
        [Event::ClipboardLoad { selection: "c".into(), terminator: Terminator::Bel, replies_before: 6 }]
    );
    assert_eq!(rig.replies(), "\x1b[1;1R\x1b[0n");
}

#[test]
fn what_the_hosts_image_code_gets_arrives_whole_whatever_the_cuts() {
    let mut rig = Rig::new(20, 5);
    // A big picture, cut at an awkward size: kitty raw RGBA, 600 by 100.
    let pixels = vec![0x5Au8; 600 * 100 * 4];
    let apc =
        format!("\x1b_Ga=t,f=32,s=600,v=100,i=1,q=2;{}\x1b\\", crate::terminal::osc::base64::encode(&pixels));
    for piece in apc.as_bytes().chunks(777) {
        rig.feed_bytes(piece);
    }
    rig.feed("\x1b_Xnot kitty\x1b\\\x1bP0;1q#0;2;0;0;0~~~~\x1b\\\x1b]1337;SetMark\x07\x1b]7501;state=working\x1b\\");
    let events = rig.events();
    assert_eq!(events.len(), 4, "{events:?}");
    assert!(
        matches!(&events[0], Event::ImageData(up) if up.data == pixels && (up.width, up.height) == (600, 100))
    );
    assert!(matches!(&events[1], Event::ImageData(up) if (up.width, up.height) == (4, 12)));
    assert_eq!(events[2], Event::Iterm(b"SetMark".to_vec()));
    assert_eq!(
        events[3],
        Event::ProgramStatus { payload: b"state=working".to_vec(), terminator: Terminator::St }
    );
}

#[test]
fn a_string_over_its_limit_is_dropped_whole() {
    let mut rig = Rig::new(20, 5);
    rig.feed(&format!("\x1b]0;{}\x07ok", "x".repeat(9000)));
    assert!(rig.events().is_empty());
    assert_eq!(rig.term.title(), None);
    assert_eq!(rig.row(0), "ok");
}

#[test]
fn shell_marks_label_the_rows_they_cover() {
    let mut rig = Rig::new(20, 6);
    rig.feed("\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07out1\r\nout2\r\n\x1b]133;D;3\x07");
    rig.feed("\x1b]133;A;k=s\x07> \x1b]133;B\x07");
    let events = rig.events();
    assert_eq!(
        events,
        [
            Event::Mark(ShellMark::PromptStart(PromptKind::Initial)),
            Event::Mark(ShellMark::CommandStart),
            Event::Mark(ShellMark::OutputStart),
            Event::Mark(ShellMark::Finished(Some(3))),
            Event::Mark(ShellMark::PromptStart(PromptKind::Secondary)),
            Event::Mark(ShellMark::CommandStart),
        ]
    );
    let mark = |rig: &Rig, y: i32| rig.term.row_mark(Line(y));
    assert_eq!(mark(&rig, 0), Some(RowMark { block: 1, prompt: true, command: true, output: false }));
    assert_eq!(mark(&rig, 1), Some(RowMark { block: 1, prompt: false, command: false, output: true }));
    assert_eq!(mark(&rig, 2), Some(RowMark { block: 1, prompt: false, command: false, output: true }));
    // A secondary prompt belongs to the same block.
    assert_eq!(mark(&rig, 3).map(|m| m.block), Some(1));
    // The next initial prompt starts the next block.
    rig.feed("\r\n\x1b]133;A\x07$ \x1b]133;B\x07");
    assert_eq!(mark(&rig, 4).map(|m| m.block), Some(2));
}

#[test]
fn what_a_full_screen_program_prints_is_not_a_block() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b]133;A\x07$ \x1b]133;B\x07vim\r\n\x1b]133;C\x07");
    rig.feed("\x1b[?1049hhello\x1b[?1049l");
    assert_eq!(rig.term.row_mark(Line(0)).map(|m| m.block), Some(1));
    // On leaving, the output mark is back in force for what the shell prints next.
    rig.feed("after");
    assert_eq!(rig.term.row_mark(Line(1)).map(|m| m.output), Some(true));
}

#[test]
fn events_do_not_pile_up_for_a_host_that_never_looks() {
    let mut rig = Rig::new(20, 5);
    for i in 0..30_000 {
        rig.feed(&format!("\x1b]9;n{i}\x07"));
    }
    assert!(rig.term.drain_events().count() <= 16_384);
}

#[test]
fn a_full_reset_is_an_event_and_forgets_the_stack() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]2;a\x07\x1b[22t\x1bc\x1b[23t");
    assert!(rig.events().contains(&Event::FullReset));
    assert_eq!(rig.term.title(), Some("a"), "RIS keeps the title; the stack is empty");
}
