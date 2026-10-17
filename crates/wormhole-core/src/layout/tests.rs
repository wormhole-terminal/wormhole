use super::{
    Columns, Edge, PANEL_DEFAULT, PANEL_MAX, PANEL_MIN, PANEL_STRIP, Panes, Resize, SIDEBAR_DEFAULT,
    SIDEBAR_MAX, SIDEBAR_MIN, Stop, TERMINAL_MIN, WINDOW_MIN_WIDTH, resize, right_panel_fits,
};

/// What the panel needs beside it: the terminal and the strip.
const MIDDLE: f32 = TERMINAL_MIN + PANEL_STRIP;
const PANEL: f32 = PANEL_MIN;

fn panes(sidebar_open: bool, sidebar_width: f32, panel_wanted: bool, panel_width: f32) -> Panes {
    Panes { sidebar_open, sidebar_width, panel_wanted, panel_width }
}

// ---------------------------------------------------------------- the fit rule

#[test]
fn fits_when_wide() {
    assert!(right_panel_fits(1320.0, Some(350.0), MIDDLE, PANEL));
}

#[test]
fn boundary_is_inclusive() {
    assert!(right_panel_fits(908.0, Some(320.0), MIDDLE, PANEL));
    assert!(!right_panel_fits(907.5, Some(320.0), MIDDLE, PANEL));
}

#[test]
fn collapsed_sidebar_frees_its_width() {
    assert!(right_panel_fits(588.0, None, MIDDLE, PANEL));
    assert!(!right_panel_fits(587.0, None, MIDDLE, PANEL));
    // The smallest window (sidebar + terminal) has room once the sidebar is gone.
    assert!(!right_panel_fits(WINDOW_MIN_WIDTH, Some(SIDEBAR_MIN), MIDDLE, PANEL));
    assert!(right_panel_fits(WINDOW_MIN_WIDTH, None, MIDDLE, PANEL));
}

#[test]
fn wider_sidebar_needs_more_room() {
    assert!(right_panel_fits(930.0, Some(320.0), MIDDLE, PANEL));
    assert!(!right_panel_fits(930.0, Some(420.0), MIDDLE, PANEL));
}

#[test]
fn very_narrow_never_fits() {
    assert!(!right_panel_fits(0.0, None, MIDDLE, PANEL));
    assert!(!right_panel_fits(300.0, None, MIDDLE, PANEL));
}

#[test]
fn the_column_sizes() {
    // With both columns draggable down to 240: sidebar 240–420 (320 by default),
    // panel 240–720 (300 by default, and its 28 strip), terminal 320.
    assert_eq!((SIDEBAR_DEFAULT, SIDEBAR_MIN, SIDEBAR_MAX), (320.0, 240.0, 420.0));
    assert_eq!((PANEL_DEFAULT, PANEL_MIN, PANEL_MAX, PANEL_STRIP), (300.0, 240.0, 720.0, 28.0));
    assert_eq!(TERMINAL_MIN, 320.0);
    // The smallest window is the narrowest sidebar, the terminal and the strip: 588.
    assert_eq!(WINDOW_MIN_WIDTH, 588.0);
    assert_eq!(Panes::default(), panes(true, 320.0, true, 300.0));
}

#[test]
fn the_panel_thresholds() {
    let open = |sidebar| panes(true, sidebar, true, PANEL_DEFAULT);
    let cols = |sidebar, terminal, panel| Columns { sidebar, terminal, panel, strip: PANEL_STRIP };
    // Sidebar 320: the panel from 908, at its full 300 from 968.
    assert!(!open(320.0).panel_visible(907.0));
    assert_eq!(open(320.0).columns(908.0), cols(320.0, 320.0, 240.0));
    assert_eq!(open(320.0).columns(968.0), cols(320.0, 320.0, 300.0));
    // Sidebar 420: from 1008, full at 1068.
    assert!(!open(420.0).panel_visible(1007.0));
    assert_eq!(open(420.0).columns(1008.0), cols(420.0, 320.0, 240.0));
    assert_eq!(open(420.0).columns(1068.0), cols(420.0, 320.0, 300.0));
    // Sidebar hidden: the panel fits the smallest window.
    let hidden = panes(false, 320.0, true, PANEL_DEFAULT);
    assert!(hidden.panel_visible(WINDOW_MIN_WIDTH));
}

#[test]
fn the_columns_at_window_widths() {
    let panes = Panes::default();
    let cols = |sidebar, terminal, panel| Columns { sidebar, terminal, panel, strip: PANEL_STRIP };
    // 1440: everything at its default, the terminal takes the rest.
    assert_eq!(panes.columns(1440.0), cols(320.0, 792.0, 300.0));
    // 960: the panel yields first, down from 300 to 292, so the terminal keeps 320.
    assert_eq!(panes.columns(960.0), cols(320.0, 320.0, 292.0));
    // 900: no room for the panel (900 < 908): it hides by itself, still wanted; the strip stays.
    assert_eq!(panes.columns(900.0), cols(320.0, 552.0, 0.0));
    assert!(panes.panel_wanted);
    // 670: sidebar and terminal, no room for the panel.
    assert_eq!(panes.columns(670.0), cols(320.0, 322.0, 0.0));
    // 670, sidebar hidden and panel closed: the terminal has the window less the strip.
    let hidden = Panes { sidebar_open: false, panel_wanted: false, ..panes };
    assert_eq!(hidden.columns(670.0), cols(0.0, 642.0, 0.0));
}

// ---------------------------------------------------------------- toggles

#[test]
fn the_panel_toggle_hides_what_is_shown() {
    let mut panes = Panes::default();
    assert!(!panes.toggle_panel(1440.0));
    assert!(!panes.panel_wanted);
    assert!(panes.toggle_panel(1440.0));
    assert!(panes.panel_wanted && panes.sidebar_open, "with room the sidebar stays");
}

#[test]
fn opening_the_panel_without_room_hides_the_sidebar() {
    // With no room, the panel wins because it was asked for.
    let mut panes = Panes { panel_wanted: false, ..Panes::default() };
    assert!(panes.toggle_panel(670.0));
    assert!(!panes.sidebar_open);
    assert_eq!(
        panes.columns(670.0),
        Columns { sidebar: 0.0, terminal: 342.0, panel: 300.0, strip: PANEL_STRIP }
    );
    // The panel hidden for room (wanted) is opened, not "closed", by the toggle.
    let mut waiting = Panes::default();
    assert!(!waiting.panel_visible(900.0));
    assert!(waiting.toggle_panel(900.0));
    assert!(!waiting.sidebar_open);
}

#[test]
fn bringing_the_sidebar_back_hides_the_panel_again() {
    let mut panes = Panes { sidebar_open: false, ..Panes::default() };
    assert!(panes.panel_visible(670.0));
    panes.toggle_sidebar();
    assert!(panes.sidebar_open);
    // The sidebar wins because it was asked for last; the panel waits for room, still wanted.
    assert!(!panes.panel_visible(670.0));
    assert!(panes.panel_wanted);
    // Widening the window brings it back by itself.
    assert!(panes.panel_visible(910.0));
    assert!(!panes.panel_visible(907.0));
}

#[test]
fn a_saved_layout_is_brought_into_range() {
    let wild = panes(true, 9000.0, true, 10.0).restored();
    assert_eq!((wild.sidebar_width, wild.panel_width), (SIDEBAR_MAX, PANEL_MIN));
    let fine = panes(false, 400.0, false, 500.0);
    assert_eq!(fine.restored(), fine);
}

// ---------------------------------------------------------------- dragging an edge

#[test]
fn a_drag_follows_the_pointer_between_the_limits() {
    assert_eq!(resize(400.0, 350.0, 420.0), Resize { open: true, width: 400.0, stop: None });
    // Past the maximum it stays there.
    assert_eq!(resize(500.0, 350.0, 420.0), Resize { open: true, width: 420.0, stop: Some(Stop::Max) });
}

#[test]
fn past_the_minimum_it_sticks_then_hides() {
    // The sidebar sticks at 240 from 240 down to 170 (240 − 70), and hides past 170.
    assert_eq!(resize(200.0, 240.0, 420.0), Resize { open: true, width: 240.0, stop: Some(Stop::Min) });
    assert_eq!(resize(170.0, 240.0, 420.0), Resize { open: true, width: 240.0, stop: Some(Stop::Min) });
    assert_eq!(resize(169.0, 240.0, 420.0), Resize { open: false, width: 240.0, stop: None });
    // The right panel: the same, sticks at 240 until 170, then closes.
    assert_eq!(resize(200.0, 240.0, 720.0), Resize { open: true, width: 240.0, stop: Some(Stop::Min) });
    assert_eq!(resize(169.0, 240.0, 720.0), Resize { open: false, width: 240.0, stop: None });
}

#[test]
fn dragging_back_reopens_at_the_minimum() {
    let mut panes = Panes::default();
    panes.drag(Edge::Sidebar, 100.0, 1440.0);
    assert!(!panes.sidebar_open);
    panes.drag(Edge::Sidebar, 200.0, 1440.0);
    assert!(panes.sidebar_open);
    assert_eq!(panes.sidebar_width, SIDEBAR_MIN);
    panes.drag(Edge::Panel, 50.0, 1440.0);
    assert!(!panes.panel_wanted);
    panes.drag(Edge::Panel, 200.0, 1440.0);
    assert!(panes.panel_wanted);
    assert_eq!(panes.panel_width, PANEL_MIN);
}

#[test]
fn a_drag_moves_the_panels_own_width() {
    let mut panes = Panes::default();
    panes.drag(Edge::Sidebar, 400.0, 1440.0);
    panes.drag(Edge::Panel, 500.0, 1440.0);
    assert_eq!(panes, self::panes(true, 400.0, true, 500.0));
    // The window can spare 1440 − 400 − 28 − 320 = 692.
    panes.drag(Edge::Panel, 900.0, 1440.0);
    assert_eq!(panes.panel_width, 692.0);
    panes.drag(Edge::Panel, 900.0, 2000.0);
    assert_eq!(panes.panel_width, PANEL_MAX);
}

// ---------------------------------------------------------------- what the window can spare

#[test]
fn the_terminal_never_goes_below_its_minimum() {
    // A wide sidebar in the smallest window gives way to its own minimum.
    let wide = panes(true, 420.0, false, PANEL_DEFAULT);
    let cols = |sidebar, terminal| Columns { sidebar, terminal, panel: 0.0, strip: PANEL_STRIP };
    assert_eq!(wide.columns(670.0), cols(322.0, 320.0));
    assert_eq!(wide.columns(768.0), cols(420.0, 320.0));
    // It keeps its chosen width for when the window grows.
    assert_eq!(wide.sidebar_width, 420.0);
    for window in [670.0, 700.0, 930.0, 1000.0, 1100.0, 1440.0] {
        for panes in
            [Panes::default(), wide, panes(true, 420.0, true, PANEL_MAX), panes(false, 350.0, true, 500.0)]
        {
            let columns = panes.columns(window);
            assert!(columns.terminal >= TERMINAL_MIN, "{window}: {columns:?}");
            assert_eq!(
                columns.sidebar + columns.terminal + columns.panel + columns.strip,
                window,
                "{window}: {columns:?}"
            );
        }
    }
}

#[test]
fn the_panel_is_dragged_no_wider_than_the_window_can_spare() {
    // A 1100 window with the sidebar open stops the panel at 1100 − 320 − 28 − 320 = 432.
    let mut panes = Panes::default();
    assert_eq!(panes.max_width(Edge::Panel, 1100.0), 432.0);
    let to = panes.drag(Edge::Panel, 600.0, 1100.0);
    assert_eq!(to, Resize { open: true, width: 432.0, stop: Some(Stop::Max) });
    assert_eq!(to.readout(Edge::Panel), "432 · max");
    // At 1440 it is the panel's own maximum, 720.
    assert_eq!(panes.max_width(Edge::Panel, 1440.0), PANEL_MAX);
    assert_eq!(panes.max_width(Edge::Panel, 2000.0), PANEL_MAX);
    // With the sidebar hidden the panel may take its room.
    panes.sidebar_open = false;
    assert_eq!(panes.max_width(Edge::Panel, 1100.0), 720.0);
    assert_eq!(panes.max_width(Edge::Panel, 900.0), 552.0);
    // Never below the minimum, however narrow.
    assert_eq!(Panes::default().max_width(Edge::Panel, 670.0), PANEL_MIN);
}

#[test]
fn the_sidebar_is_dragged_no_wider_than_the_terminal_allows() {
    let mut panes = Panes::default();
    assert_eq!(panes.max_width(Edge::Sidebar, 1440.0), SIDEBAR_MAX);
    assert_eq!(panes.max_width(Edge::Sidebar, 700.0), 352.0);
    assert_eq!(panes.max_width(Edge::Sidebar, 670.0), 322.0);
    assert_eq!(panes.max_width(Edge::Sidebar, WINDOW_MIN_WIDTH), SIDEBAR_MIN);
    let to = panes.drag(Edge::Sidebar, 500.0, 700.0);
    assert_eq!((to.width, to.readout(Edge::Sidebar)), (352.0, "352 · max".to_string()));
    // The pointer between 420 and 500 in a wide window: stays at 420.
    assert_eq!(panes.drag(Edge::Sidebar, 470.0, 1440.0).readout(Edge::Sidebar), "420 · max");
}

#[test]
fn the_readout_says_where_the_drag_stopped() {
    let mut panes = Panes::default();
    assert_eq!(panes.drag(Edge::Sidebar, 400.4, 1440.0).readout(Edge::Sidebar), "400");
    assert_eq!(panes.drag(Edge::Sidebar, 200.0, 1440.0).readout(Edge::Sidebar), "240 · min");
    // Past min − 70 the sidebar hides (there is no rail in v1) and the panel closes.
    assert_eq!(panes.drag(Edge::Sidebar, 169.0, 1440.0).readout(Edge::Sidebar), "Hide");
    assert_eq!(panes.drag(Edge::Panel, 300.0, 1440.0).readout(Edge::Panel), "300");
    assert_eq!(panes.drag(Edge::Panel, 200.0, 1440.0).readout(Edge::Panel), "240 · min");
    assert_eq!(panes.drag(Edge::Panel, 169.0, 1440.0).readout(Edge::Panel), "Close");
}

#[test]
fn a_double_click_puts_the_default_width_back() {
    let mut panes = panes(true, 410.0, true, 640.0);
    panes.reset(Edge::Sidebar);
    assert_eq!(panes.sidebar_width, 320.0);
    panes.reset(Edge::Panel);
    assert_eq!(panes.panel_width, 300.0);
    assert_eq!(panes, Panes::default());
}
