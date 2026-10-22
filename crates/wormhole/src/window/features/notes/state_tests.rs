//! What a terminal's notes and tasks do through the window, pinned from outside: saved with the
//! terminal and read back, counted for its row and the close question, kept when the terminal moves
//! to another window or the feature is off, let go of when another terminal is chosen, and walked by
//! the keys. They reach the state only through the helpers at the top, so they hold wherever it lives.
use gpui_kit::{App, Context, TestAppContext, Window};
use wormhole_core::config::TabId;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::store::{NoteRecord, SessionRecord, TaskRecord};
use wormhole_core::view::focus::Area;

use super::testing as notes;
use crate::app_state::{OpenWindow, app};
use crate::settings::update_settings;
use crate::window::Workspace;
use crate::window::view_tests::read;
use crate::window::view_tests::{draw, drive, isolate_in, press, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A terminal's notes and tasks as saved: the part of its record this feature writes.
#[derive(Debug, PartialEq)]
struct Saved {
    tasks: Vec<(String, bool)>,
    notes: Vec<(String, bool)>,
    tasks_collapsed: bool,
    notes_collapsed: bool,
}

impl Saved {
    fn of(record: &SessionRecord) -> Self {
        Self {
            tasks: record.tasks.iter().map(|t| (t.text.clone(), t.done)).collect(),
            notes: record.notes.iter().map(|n| (n.body.clone(), n.collapsed)).collect(),
            tasks_collapsed: record.tasks_collapsed,
            notes_collapsed: record.notes_collapsed,
        }
    }
}

fn record_with(
    id: i64,
    tasks: &[(&str, bool)],
    notes: &[(&str, bool)],
    folds: (bool, bool),
) -> SessionRecord {
    let mut record = SessionRecord::new(id, None, None);
    record.tasks = tasks.iter().map(|&(text, done)| TaskRecord { text: text.into(), done }).collect();
    record.notes =
        notes.iter().map(|&(body, collapsed)| NoteRecord { body: body.into(), collapsed }).collect();
    (record.tasks_collapsed, record.notes_collapsed) = folds;
    record
}

/// What the window would save for the terminal with this id.
fn saved(ws: &Workspace, id: i64, cx: &App) -> Saved {
    let window_record = ws.window_record(cx);
    let record = window_record.sessions.iter().find(|s| s.id == id).expect("a terminal of this window");
    Saved::of(record)
}

/// Opens a terminal in the window from `record`, as a restart or a move does.
fn open(ws: &mut Workspace, record: SessionRecord, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.add_session(Some(record), window, cx);
}

/// The notes and tasks the terminal's row counts and the close question names.
fn open_items(ws: &Workspace, id: i64, cx: &App) -> usize {
    // Off, nothing is at stake: the close question asks only the features that are on.
    ws.close_stakes(&[id], cx).first().map_or(0, |stake| stake.count)
}

/// The note ⌘↵ and the stash act on, in the selected terminal.
fn focused_note(ws: &Workspace, cx: &App) -> Option<usize> {
    notes::focused_note(ws, cx)
}

fn set_flag(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::NotesAndTasks.key().to_string(), on);
        })
    });
}

fn key(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    draw(cx, window);
    press(cx, window, keys);
}

#[gpui_kit::test]
fn a_terminals_notes_and_tasks_are_saved_with_it_and_read_back_as_they_were(cx: &mut TestAppContext) {
    if isolate("a_terminals_notes_and_tasks_are_saved_with_it_and_read_back_as_they_were") {
        return;
    }
    let window = start(cx, 1);
    let first = record_with(
        11,
        &[("ship it", false), ("done already", true)],
        &[("one\nsecond line", false), ("two", true)],
        (true, false),
    );
    let second = record_with(12, &[("other", false)], &[("only", false)], (false, true));
    let (a, b, untouched) = drive(cx, &window, |ws, window, cx| {
        open(ws, first, window, cx);
        open(ws, second, window, cx);
        (saved(ws, 11, cx), saved(ws, 12, cx), saved(ws, 1, cx))
    });
    let tasks = |items: &[(&str, bool)]| items.iter().map(|&(t, d)| (t.to_string(), d)).collect::<Vec<_>>();
    assert_eq!(a.tasks, tasks(&[("ship it", false), ("done already", true)]));
    assert_eq!(a.notes, tasks(&[("one\nsecond line", false), ("two", true)]));
    assert_eq!((a.tasks_collapsed, a.notes_collapsed), (true, false));
    assert_eq!(b.tasks, tasks(&[("other", false)]));
    assert_eq!(b.notes, tasks(&[("only", false)]));
    assert_eq!((b.tasks_collapsed, b.notes_collapsed), (false, true));
    assert_eq!(
        untouched,
        Saved { tasks: vec![], notes: vec![], tasks_collapsed: false, notes_collapsed: false }
    );
}

#[gpui_kit::test]
fn the_row_and_the_close_question_count_notes_with_text_and_tasks_not_done(cx: &mut TestAppContext) {
    if isolate("the_row_and_the_close_question_count_notes_with_text_and_tasks_not_done") {
        return;
    }
    let window = start(cx, 1);
    let record = record_with(
        11,
        &[("a", false), ("b", true), ("c", false)],
        &[("text", false), ("  \n ", false), ("more", true)],
        (false, false),
    );
    let counts = drive(cx, &window, |ws, window, cx| {
        open(ws, record, window, cx);
        (open_items(ws, 1, cx), open_items(ws, 11, cx))
    });
    assert_eq!(counts, (0, 4), "two open tasks and two notes with something in them");

    // Off, the row shows nothing and the question says nothing of notes; the data stays.
    set_flag(cx, false);
    assert_eq!(drive(cx, &window, |ws, _, cx| open_items(ws, 11, cx)), 0);
    set_flag(cx, true);
    assert_eq!(drive(cx, &window, |ws, _, cx| open_items(ws, 11, cx)), 4);
}

#[gpui_kit::test]
fn setting_a_notes_text_from_outside_updates_the_count_and_the_save(cx: &mut TestAppContext) {
    if isolate("setting_a_notes_text_from_outside_updates_the_count_and_the_save") {
        return;
    }
    let window = start(cx, 1);
    let counted = |cx: &mut TestAppContext, text: &str| {
        drive(cx, &window, |ws, window, cx| {
            assert!(notes::set_note_text(ws, 0, 0, text, window, cx), "the note is there");
            (open_items(ws, 1, cx), saved(ws, 1, cx).notes.len(), ws.close_stakes(&[1], cx).len())
        })
    };
    drive(cx, &window, |ws, window, cx| {
        notes::add_note(ws, 0, "", false, window, cx);
    });
    assert_eq!(drive(cx, &window, |ws, _, cx| open_items(ws, 1, cx)), 0, "a blank note counts for nothing");
    // `set_value` sends no event of its own: the count, the row and the close question follow anyway.
    assert_eq!(counted(cx, "written by devctl"), (1, 1, 1));
    assert_eq!(counted(cx, "  "), (0, 1, 0), "blank again: not counted, and still a note");
    let missing = drive(cx, &window, |ws, window, cx| notes::set_note_text(ws, 0, 5, "x", window, cx));
    assert!(!missing, "no such note");
}

#[gpui_kit::test]
fn switching_the_feature_off_keeps_what_is_saved_and_on_again_brings_it_back(cx: &mut TestAppContext) {
    if isolate("switching_the_feature_off_keeps_what_is_saved_and_on_again_brings_it_back") {
        return;
    }
    let window = start(cx, 1);
    let record = record_with(11, &[("kept", false)], &[("also kept", false)], (false, false));
    let before = drive(cx, &window, |ws, window, cx| {
        open(ws, record, window, cx);
        saved(ws, 11, cx)
    });
    set_flag(cx, false);
    draw(cx, &window);
    assert_eq!(drive(cx, &window, |ws, _, cx| saved(ws, 11, cx)), before, "off: it is still saved");
    set_flag(cx, true);
    draw(cx, &window);
    assert_eq!(drive(cx, &window, |ws, _, cx| saved(ws, 11, cx)), before, "and on again, still there");
}

#[gpui_kit::test]
fn a_terminal_moved_to_another_window_keeps_its_notes_and_tasks(cx: &mut TestAppContext) {
    if isolate("a_terminal_moved_to_another_window_keeps_its_notes_and_tasks") {
        return;
    }
    let window = start(cx, 1);
    let record = record_with(11, &[("ship it", false)], &[("remember", true)], (true, false));
    let (group, before) = drive(cx, &window, |ws, window, cx| {
        open(ws, record, window, cx);
        (ws.group_terminals(&[11], cx), saved(ws, 11, cx))
    });
    drive(cx, &window, |ws, window, cx| ws.move_group_to_window(group, window, cx));
    cx.run_until_parked();
    let other = cx
        .update(|cx| app(cx).windows.iter().find(|w| w.id != window.id).cloned())
        .expect("a window of its own");
    let after = drive(cx, &other, |ws, _, cx| saved(ws, 11, cx));
    assert_eq!(after, before, "the same notes and tasks, in the same folds");
    let here: Vec<i64> = read(cx, &window, |ws| ws.sessions.iter().map(|s| s.id).collect());
    assert_eq!(here, [1], "and the first window no longer has the terminal");
}

#[gpui_kit::test]
fn choosing_another_terminal_lets_go_of_the_focused_note(cx: &mut TestAppContext) {
    if isolate("choosing_another_terminal_lets_go_of_the_focused_note") {
        return;
    }
    let window = start(cx, 2);
    drive(cx, &window, |ws, window, cx| notes::new_note(ws, ws.active, Some("first"), window, cx));
    assert_eq!(read_cx(cx, &window, focused_note), Some(0));
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert_eq!(read_cx(cx, &window, focused_note), None, "the other terminal starts fresh");
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    assert_eq!(read_cx(cx, &window, focused_note), None, "and so does coming back");
}

#[gpui_kit::test]
fn the_notes_list_is_walked_by_the_keys_and_a_note_is_folded_or_written_in(cx: &mut TestAppContext) {
    if isolate("the_notes_list_is_walked_by_the_keys_and_a_note_is_folded_or_written_in") {
        return;
    }
    let window = start(cx, 1);
    let record =
        record_with(11, &[("a", false), ("b", false)], &[("n1", false), ("n2", false)], (false, false));
    drive(cx, &window, |ws, window, cx| {
        open(ws, record, window, cx);
        let ix = ws.find_session(11).expect("opened");
        ws.activate(ix, window, cx);
        ws.show_tab(TabId::Notes, cx);
    });
    draw(cx, &window);
    // Two lists, one ⇥ stop each: Next steps (its band, a, b) and Notes (its band, n1, n2).
    assert!(read(cx, &window, |ws| ws.areas.panel_at.is_none()));
    assert_eq!(drive(cx, &window, |ws, _, cx| ws.panel_lists(cx)), [0, 3]);

    let area = |cx: &mut TestAppContext| {
        draw(cx, &window);
        cx.update(|cx| {
            window.handle.update(cx, |_, win, cx| {
                window.workspace.upgrade().and_then(|workspace| workspace.read(cx).focus_area(win, cx))
            })
        })
        .ok()
        .flatten()
    };
    for _ in 0..Area::ALL.len() {
        if area(cx) == Some(Area::PanelContent) {
            break;
        }
        key(cx, &window, "f6");
    }
    assert_eq!(area(cx), Some(Area::PanelContent));
    // Down over the steps and the Notes band onto the first note; Space folds it.
    for _ in 0..4 {
        key(cx, &window, "down");
    }
    key(cx, &window, "space");
    assert_eq!(
        drive(cx, &window, |ws, _, cx| saved(ws, 11, cx).notes),
        [("n1".into(), true), ("n2".into(), false)]
    );
    assert_eq!(area(cx), Some(Area::PanelContent), "the list keeps the keyboard");
    // Return on the next note writes in it: the keys go to its editor and it is the focused note.
    key(cx, &window, "down");
    key(cx, &window, "enter");
    assert_eq!(read_cx(cx, &window, focused_note), Some(1));
    assert_eq!(area(cx), Some(Area::PanelContent), "the editor holds the keys, in the panel");
}

#[gpui_kit::test]
fn cmd_shift_e_with_no_note_starts_one_and_with_notes_goes_to_the_one_worked_on(cx: &mut TestAppContext) {
    if isolate("cmd_shift_e_with_no_note_starts_one_and_with_notes_goes_to_the_one_worked_on") {
        return;
    }
    let window = start(cx, 1);
    let toggle = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, window, cx| {
            ws.focus_terminal(window, cx);
            ws.toggle_focus(window, cx);
        });
        drive(cx, &window, |ws, _, cx| (ws.right_tab, saved(ws, 1, cx).notes.len(), focused_note(ws, cx)))
    };
    // An empty note is thrown away when it loses the keyboard, so the first press makes one and the
    // next, with the terminal focused again, finds none and makes another.
    assert_eq!(toggle(cx), (TabId::Notes, 1, Some(0)), "a new note, on the Notes tab");
    let record = record_with(11, &[], &[("first", false), ("second", false)], (false, false));
    drive(cx, &window, |ws, window, cx| {
        open(ws, record, window, cx);
        let ix = ws.find_session(11).expect("opened");
        ws.activate(ix, window, cx);
        ws.show_tab(TabId::Changes, cx);
    });
    let toggle_in_11 = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, window, cx| {
            ws.focus_terminal(window, cx);
            ws.toggle_focus(window, cx);
        });
        drive(cx, &window, |ws, _, cx| (ws.right_tab, saved(ws, 11, cx).notes.len(), focused_note(ws, cx)))
    };
    assert_eq!(toggle_in_11(cx), (TabId::Notes, 2, Some(0)), "the first note when none was worked on");
    drive(cx, &window, |ws, _, cx| notes::set_focused_note(ws, Some(1), cx));
    assert_eq!(toggle_in_11(cx), (TabId::Notes, 2, Some(1)), "back into the note that was worked on");
}

#[gpui_kit::test]
fn the_notes_keep_an_entry_for_exactly_the_terminals_of_the_window(cx: &mut TestAppContext) {
    if isolate("the_notes_keep_an_entry_for_exactly_the_terminals_of_the_window") {
        return;
    }
    let window = start(cx, 2);
    let ids = |cx: &mut TestAppContext, window: &OpenWindow| {
        read_cx(cx, window, |ws, cx| {
            let mut ours: Vec<i64> = ws.sessions.iter().map(|s| s.id).collect();
            ours.sort_unstable();
            (notes::kept_ids(ws, cx), ours)
        })
    };
    let (kept, ours) = ids(cx, &window);
    assert_eq!(kept, ours, "made with the window's terminals");

    drive(cx, &window, |ws, window, cx| {
        open(ws, record_with(11, &[("a", false)], &[("n", false)], (false, false)), window, cx);
    });
    let (kept, ours) = ids(cx, &window);
    assert_eq!((kept.clone(), kept.len()), (ours, 3), "added with a new terminal");

    drive(cx, &window, |ws, window, cx| {
        let ix = ws.find_session(11).expect("a row");
        ws.close_session(ix, window, cx);
    });
    let (kept, ours) = ids(cx, &window);
    assert_eq!((kept.clone(), kept.len()), (ours, 2), "dropped with a closed terminal");

    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2], cx));
    drive(cx, &window, |ws, window, cx| ws.move_group_to_window(group, window, cx));
    cx.run_until_parked();
    let (kept, ours) = ids(cx, &window);
    assert_eq!((kept.clone(), kept.len()), (ours, 1), "and with a terminal that moved away");
}

#[gpui_kit::test]
fn a_terminals_row_counts_what_is_open_and_the_close_question_says_it_is_discarded(cx: &mut TestAppContext) {
    use crate::window::sidebar::{RowExtra, RowExtras, RowInfo};
    if isolate("a_terminals_row_counts_what_is_open_and_the_close_question_says_it_is_discarded") {
        return;
    }
    let window = start(cx, 1);
    let record =
        record_with(11, &[("a", false), ("b", true)], &[("text", false), (" ", false)], (false, false));
    let (extras, stake) = drive(cx, &window, |ws, window, cx| {
        open(ws, record, window, cx);
        let mut rows = vec![RowInfo::test_row(1), RowInfo::test_row(11)];
        ws.feature_row_extras(&mut rows, cx);
        let extras: Vec<_> = RowExtras::new(&mut rows).iter_mut().map(|(id, slot)| (id, *slot)).collect();
        (extras, ws.close_stakes(&[1, 11], cx))
    });
    assert_eq!(extras, [(1, None), (11, Some(RowExtra { count: 2 }))], "an open task and a note with text");
    assert_eq!(stake.len(), 1);
    assert_eq!((stake[0].count, stake[0].sentence.as_str()), (2, "notes and tasks are discarded."));
    // Off, the row and the question say nothing.
    set_flag(cx, false);
    let (extras, stake) = drive(cx, &window, |ws, _, cx| {
        let mut rows = vec![RowInfo::test_row(11)];
        ws.feature_row_extras(&mut rows, cx);
        (
            RowExtras::new(&mut rows).iter_mut().next().and_then(|(_, slot)| *slot),
            ws.close_stakes(&[11], cx).len(),
        )
    });
    assert_eq!((extras, stake), (None, 0));
}

#[gpui_kit::test]
fn the_tick_throws_away_a_note_left_empty_that_never_had_the_keyboard(cx: &mut TestAppContext) {
    if isolate("the_tick_throws_away_a_note_left_empty_that_never_had_the_keyboard") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| {
        notes::add_note(ws, 0, "", false, window, cx);
        notes::add_note(ws, 0, "kept", false, window, cx);
        assert_eq!(notes::note_count(ws, 0, cx), 2);
        ws.tick_features(window, cx);
        assert_eq!(notes::note_count(ws, 0, cx), 1, "the empty one is gone");
    });
}
