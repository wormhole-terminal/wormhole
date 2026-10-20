//! The window operations as events, and what the host's geometry changes in the reports.

use super::Rig;
use crate::terminal::{Event, Extent, Fullscreen, Maximize, WindowGeometry, WindowOp, WindowOps};

fn allowed() -> Rig {
    let mut rig = Rig::new(20, 5);
    rig.term.set_window_ops(WindowOps::Allow);
    rig
}

fn ops(rig: &mut Rig) -> Vec<WindowOp> {
    rig.events()
        .into_iter()
        .filter_map(|event| match event {
            Event::WindowOp(op) => Some(op),
            _ => None,
        })
        .collect()
}

#[test]
fn every_operation_is_an_event_and_none_without_the_host() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[2t\x1b[3;10;20t\x1b[8;10;40t\x1b[9;1t\x1b[10;1t\x1b[40t");
    assert!(ops(&mut rig).is_empty());
    assert_eq!((rig.term.columns(), rig.term.screen_lines()), (20, 5));

    let mut rig = allowed();
    rig.feed("\x1b[2t\x1b[1t\x1b[5t\x1b[6t\x1b[3;10;20t\x1b[3;7t\x1b[9;1t\x1b[9;0t\x1b[10;2t\x1b[7t");
    assert_eq!(
        ops(&mut rig),
        [
            WindowOp::Iconify,
            WindowOp::Deiconify,
            WindowOp::Raise,
            WindowOp::Lower,
            WindowOp::Move { x: 10, y: 20 },
            WindowOp::Move { x: 7, y: 0 },
            WindowOp::Maximize(Maximize::Both),
            WindowOp::Maximize(Maximize::Restore),
            WindowOp::Fullscreen(Fullscreen::Toggle),
        ],
        "refresh (7) is nothing to do"
    );
}

#[test]
fn a_resize_in_cells_changes_the_engine_at_once_and_pixels_become_cells() {
    let mut rig = allowed();
    rig.term.set_cell_size(10, 20);
    rig.feed("\x1b[8;7;30t");
    assert_eq!((rig.term.columns(), rig.term.screen_lines()), (30, 7));
    assert_eq!(ops(&mut rig), [WindowOp::ResizeCells { rows: Extent::Size(7), cols: Extent::Size(30) }]);
    // 4 takes height then width in pixels; a left-out one is left alone, a 0 is the most.
    rig.feed("\x1b[4;100;150t");
    assert_eq!((rig.term.columns(), rig.term.screen_lines()), (15, 5));
    rig.feed("\x1b[4;;300t");
    assert_eq!((rig.term.columns(), rig.term.screen_lines()), (30, 5));
    let largest = ops(&mut rig);
    assert_eq!(largest.len(), 2);
    rig.feed("\x1b[8;0;0t");
    assert_eq!(
        (rig.term.columns(), rig.term.screen_lines()),
        (30, 5),
        "the host decides how large is the most"
    );
    assert_eq!(ops(&mut rig), [WindowOp::ResizeCells { rows: Extent::Largest, cols: Extent::Largest }]);
}

#[test]
fn the_hosts_geometry_is_what_the_reports_say() {
    let mut rig = Rig::new(20, 5);
    rig.term.set_cell_size(10, 20);
    rig.term.set_window(WindowGeometry {
        x: 30,
        y: 40,
        width: 500,
        height: 300,
        screen_width: 1600,
        screen_height: 900,
        max_cols: 150,
        max_rows: 40,
        iconified: true,
    });
    rig.feed("\x1b[11t\x1b[13t\x1b[14t\x1b[14;2t\x1b[15t\x1b[19t");
    assert_eq!(
        rig.replies(),
        "\x1b[2t\x1b[3;30;40t\x1b[4;100;200t\x1b[4;300;500t\x1b[5;900;1600t\x1b[9;40;150t"
    );
}

#[test]
fn the_window_size_reported_right_after_a_resize_is_what_it_will_be() {
    let mut rig = allowed();
    rig.term.set_cell_size(10, 20);
    rig.term.set_window(WindowGeometry {
        x: 0,
        y: 0,
        width: 500,
        height: 300,
        screen_width: 1600,
        screen_height: 900,
        max_cols: 150,
        max_rows: 40,
        iconified: false,
    });
    // 20 by 5 cells now; the window grows by what the text area does, the frame stays.
    rig.feed("\x1b[8;10;30t\x1b[14;2t");
    assert_eq!(rig.replies(), "\x1b[4;400;600t");
    rig.feed("\x1b[8;5;10t\x1b[14;2t");
    assert_eq!(rig.replies(), "\x1b[4;300;400t");
}

#[test]
fn a_size_past_the_screen_is_cut_to_it() {
    let mut rig = allowed();
    rig.term.set_window(WindowGeometry {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        screen_width: 1000,
        screen_height: 1000,
        max_cols: 100,
        max_rows: 30,
        iconified: false,
    });
    rig.feed("\x1b[8;9999;9999t");
    assert_eq!((rig.term.columns(), rig.term.screen_lines()), (100, 30));
}

#[test]
fn hostile_numbers_do_not_panic() {
    let mut rig = allowed();
    rig.feed("\x1b[8;65535;65535t\x1b[4;65535;65535t\x1b[65535t\x1b[9;99t\x1b[10;99t\x1b[3;65535;65535t");
    assert!(rig.term.columns() <= 500 && rig.term.screen_lines() <= 500);
}
