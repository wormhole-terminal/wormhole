//! What a program may do to the window (xterm's window operations, `allow_window_ops`) and what
//! it may ask of the window's place and size (always).
//!
//! A terminal is one pane of a window with a sidebar, a panel and a title bar around it, and
//! perhaps other panes beside it, so the operations are read as being about the pane that asked:
//! "resize to 40 rows" resizes the window so that this pane's text area gets 40 rows (the window
//! grows or shrinks by the difference, the frame around the pane stays as it is; with a split
//! the other panes follow the window, so the pane gets close to it, not exactly), "the screen
//! in pixels" is the screen's usable area (a window cannot be over the menu bar and the Dock),
//! "the screen in characters" is the largest the pane's text area could be in this window (see
//! [`geometry`]), and "maximise" is the window filling the usable area, which gives the pane that
//! size. A program sends its next operation or report at once, long before the window and the
//! layout have followed a resize: the engine answers the size reports for the size it was asked
//! for, and the pane's size here is the one asked for until the layout shows it. Positions are in points from the top left corner of the screen the window is on.
//!
//! The engine has already taken the sizes in cells (see `wormhole_vt::Policy::window_ops`); the pane
//! is resized by the window's layout, and the caller asks the terminal to bring the engine to the
//! pane's size (`Pty::sync_size`) once that is done.

use std::collections::HashMap;

use gpui_kit::*;
use wormhole_core::terminal::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};

/// The pane the operation is about: its cell and its size in cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneSize {
    pub cell: (f32, f32),
    pub cols: usize,
    pub rows: usize,
    /// What is around the pane's text area in the window's content: the sidebar, the title bar,
    /// the panel (and the other panes of a split).
    pub frame: (f32, f32),
}

/// What an operation did to the pane's size, which the layout takes a frame to show.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SizeGoal {
    /// Nothing: whatever was asked before still stands.
    #[default]
    Unchanged,
    /// The size is whatever the window ends up with.
    Forget,
    /// The window was asked to give the pane this many cells.
    Set { cols: usize, rows: usize },
}

/// What [`apply`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Applied {
    /// Whether the window is minimised afterwards, when the operation changes that.
    pub minimised: Option<bool>,
    pub size: SizeGoal,
}

/// The sizes windows had before a maximise, to go back to (XTWINOPS 9;0).
#[derive(Default)]
struct Saved(HashMap<WindowId, Size<Pixels>>);

impl Global for Saved {}

/// The screen the window is on: all of it, and the part of it a window can use (not under the
/// menu bar or the Dock), in global points.
fn screen(window: &Window, cx: &App) -> Option<(Bounds<Pixels>, Bounds<Pixels>)> {
    let display = window.display(cx)?;
    Some((display.bounds(), display.visible_bounds()))
}

/// The most cells the pane's text area could have, as `(cols, rows)`: in the screen's usable
/// area (what a maximised window gets, and the most a window can be: AppKit keeps it off the
/// menu bar and the Dock) and in full screen (over the Dock). The window's frame is taken as it
/// is now.
fn reach(window: &Window, cx: &App, pane: PaneSize) -> Option<[(u16, u16); 2]> {
    let (full, usable) = screen(window, cx)?;
    let (frame_w, frame_h) = pane.frame;
    let most = |space: Pixels, frame: f32, cell: f32, now: usize| {
        let cells = ((f32::from(space) - frame) / cell.max(1.0)).floor().max(0.0) as usize;
        cells.max(now).min(usize::from(u16::MAX)) as u16
    };
    // Full screen keeps the menu bar's strip at the top (the notch's safe area) and only gives up
    // the Dock's.
    let below_bar = full.size.height - usable.origin.y;
    Some([
        (
            most(usable.size.width, frame_w, pane.cell.0, pane.cols),
            most(usable.size.height, frame_h, pane.cell.1, pane.rows),
        ),
        (
            most(full.size.width, frame_w, pane.cell.0, pane.cols),
            most(below_bar, frame_h, pane.cell.1, pane.rows),
        ),
    ])
}

/// Where the window is and how large everything is, for the reports a program asks for.
/// `None` when the window is not on a screen.
///
/// The screen is the part of it a window can use, so that a window made as large as the screen
/// (XTWINOPS 4 with a 0) is as large as the report says. "The screen in characters" (XTWINOPS 19)
/// is the most the text area can have right now: what a maximised window gets (the screen less
/// the menu bar and the Dock), and in full screen the whole screen. It is always a size the
/// window really reaches, never an average of the two.
pub fn geometry(window: &Window, cx: &App, pane: PaneSize) -> Option<WindowGeometry> {
    let (full, usable) = screen(window, cx)?;
    let bounds = window.bounds();
    let [usable_cells, full_cells] = reach(window, cx, pane)?;
    let (max_cols, max_rows) = if window.is_fullscreen() { full_cells } else { usable_cells };
    Some(WindowGeometry {
        x: f32::from(bounds.origin.x - full.origin.x).round() as i32,
        y: f32::from(bounds.origin.y - full.origin.y).round() as i32,
        width: f32::from(bounds.size.width).round() as u32,
        height: f32::from(bounds.size.height).round() as u32,
        screen_width: f32::from(usable.size.width).round() as u32,
        screen_height: f32::from(usable.size.height).round() as u32,
        max_cols,
        max_rows,
        iconified: is_minimised(window),
    })
}

/// Carry out `op` for the window around the pane `pane`.
pub fn apply(op: WindowOp, pane: PaneSize, window: &mut Window, cx: &mut App) -> Applied {
    let mut done = Applied::default();
    match op {
        WindowOp::Iconify => {
            window.minimize_window();
            done.minimised = Some(true);
        }
        WindowOp::Deiconify => {
            deminiaturize(window);
            done.minimised = Some(false);
        }
        // A test instance stays behind the user's windows: `background` decides what raising does.
        WindowOp::Raise => crate::background::activate_window(window, cx),
        WindowOp::Lower => order_back(window),
        WindowOp::Move { x, y } => move_to(window, cx, x, y),
        WindowOp::ResizeCells { rows, cols } => {
            if !window.is_fullscreen() {
                done.size = resize_cells(window, cx, pane, rows, cols);
            }
        }
        WindowOp::Maximize(how) => {
            maximize(window, cx, how);
            done.size = SizeGoal::Forget;
        }
        WindowOp::Fullscreen(how) => {
            let on = window.is_fullscreen();
            let want = match how {
                Fullscreen::Enter => true,
                Fullscreen::Exit => false,
                Fullscreen::Toggle => !on,
            };
            if want != on {
                allow_fullscreen(window);
                window.toggle_fullscreen();
                done.size = SizeGoal::Forget;
            }
        }
        // An operation a newer engine adds (`WindowOp` is non-exhaustive) is one this window cannot do.
        _ => {}
    }
    done
}

/// The window's top left corner to `x`, `y` of its screen, kept inside the usable part of it.
fn move_to(window: &mut Window, cx: &App, x: i32, y: i32) {
    let Some((full, usable)) = screen(window, cx) else { return };
    let bounds = window.bounds();
    // Where the corner can be: from the usable area's corner to as far as keeps the window in it.
    let place = |wanted: i32, screen_from: Pixels, from: Pixels, space: Pixels, size: Pixels| {
        let low = f32::from(from);
        let high = low + (f32::from(space) - f32::from(size)).max(0.0);
        (f32::from(screen_from) + wanted as f32).clamp(low, high)
    };
    let x = place(x, full.origin.x, usable.origin.x, usable.size.width, bounds.size.width);
    let y = place(y, full.origin.y, usable.origin.y, usable.size.height, bounds.size.height);
    set_top_left(window, cx, x, y);
}

/// Resize the window so that the pane's text area is `rows` by `cols`.
fn resize_cells(window: &mut Window, cx: &App, pane: PaneSize, rows: Extent, cols: Extent) -> SizeGoal {
    // As large as a window can be; the engine, which took the sizes as the reports say, is brought
    // back to the pane's once the window has had its say.
    let most = reach(window, cx, pane)
        .map_or((pane.cols, pane.rows), |[usable, _]| (usize::from(usable.0), usize::from(usable.1)));
    let target = |extent: Extent, now: usize, max: usize| match extent {
        Extent::Keep => now,
        Extent::Largest => max,
        Extent::Size(n) => (n as usize).clamp(1, max.max(now)),
    };
    let (cols, rows) = (target(cols, pane.cols, most.0), target(rows, pane.rows, most.1));
    if (cols, rows) == (pane.cols, pane.rows) {
        return SizeGoal::Unchanged;
    }
    // Half a cell over, so that the pixels AppKit rounds the window to do not cost a cell.
    let width = pane.frame.0 + (cols as f32 + 0.5) * pane.cell.0;
    let height = pane.frame.1 + (rows as f32 + 0.5) * pane.cell.1;
    // AppKit shrinks a window to fit the screen from where it is, so it is moved first.
    fit_on_screen(window, cx, width, height);
    window.resize(size(px(width), px(height)));
    SizeGoal::Set { cols, rows }
}

/// Move the window, which is about to get this size, so that all of it is on the screen: below the
/// menu bar, and over the Dock rather than off the screen.
fn fit_on_screen(window: &Window, cx: &App, width: f32, height: f32) {
    let Some((full, usable)) = screen(window, cx) else { return };
    let at = window.bounds().origin;
    let place = |now: Pixels, low: Pixels, end: Pixels, extent: f32| {
        let low = f32::from(low);
        f32::from(now).clamp(low, (f32::from(end) - extent).max(low))
    };
    let x = place(at.x, usable.origin.x, full.right(), width);
    let y = place(at.y, usable.origin.y, full.bottom(), height);
    if (x, y) != (f32::from(at.x), f32::from(at.y)) {
        set_top_left(window, cx, x, y);
    }
}

/// XTWINOPS 9: fill the screen's usable area, or the width or the height of it, or go back.
fn maximize(window: &mut Window, cx: &mut App, how: Maximize) {
    let id = window.window_handle().window_id();
    let Some((_, usable)) = screen(window, cx) else { return };
    let before = window.bounds().size;
    match how {
        Maximize::Both => {
            if !window.is_maximized() {
                cx.default_global::<Saved>().0.entry(id).or_insert(before);
                window.zoom_window();
            }
        }
        Maximize::Vertical | Maximize::Horizontal => {
            cx.default_global::<Saved>().0.entry(id).or_insert(before);
            let width = if how == Maximize::Horizontal { usable.size.width } else { before.width };
            let height = if how == Maximize::Vertical { usable.size.height } else { before.height };
            fit_on_screen(window, cx, f32::from(width), f32::from(height));
            window.resize(size(width, height));
        }
        Maximize::Restore => {
            if window.is_maximized() {
                window.zoom_window();
            } else if let Some(saved) = cx.default_global::<Saved>().0.get(&id).copied() {
                window.resize(saved);
            }
            cx.default_global::<Saved>().0.remove(&id);
        }
    }
}

#[cfg(target_os = "macos")]
fn set_top_left(window: &Window, cx: &App, x: f32, y: f32) {
    use objc2_foundation::NSPoint;

    let Some(ns_window) = crate::platform::macos::appkit::ns_window(window) else { return };
    // AppKit counts y up from the bottom of the primary screen; GPUI down from its top.
    let primary = cx.displays().into_iter().find(|d| d.bounds().origin == point(px(0.0), px(0.0)));
    let Some(primary) = primary else { return };
    let top = f64::from(f32::from(primary.bounds().size.height)) - f64::from(y);
    ns_window.setFrameTopLeftPoint(NSPoint::new(f64::from(x), top));
}

#[cfg(not(target_os = "macos"))]
fn set_top_left(_: &Window, _: &App, _: f32, _: f32) {}

/// Whether the window is in the Dock. AppKit says so only once the minimise animation is done.
#[cfg(target_os = "macos")]
pub fn is_minimised(window: &Window) -> bool {
    crate::platform::macos::appkit::ns_window(window).is_some_and(|w| w.isMiniaturized())
}

#[cfg(not(target_os = "macos"))]
pub fn is_minimised(_: &Window) -> bool {
    false
}

/// A window only enters full screen (XTWINOPS 10) when it says it may: GPUI's windows do not.
#[cfg(target_os = "macos")]
fn allow_fullscreen(window: &Window) {
    use objc2_app_kit::NSWindowCollectionBehavior;
    if let Some(ns_window) = crate::platform::macos::appkit::ns_window(window) {
        ns_window.setCollectionBehavior(
            ns_window.collectionBehavior() | NSWindowCollectionBehavior::FullScreenPrimary,
        );
    }
}

#[cfg(not(target_os = "macos"))]
fn allow_fullscreen(_: &Window) {}

#[cfg(target_os = "macos")]
fn deminiaturize(window: &Window) {
    if let Some(ns_window) = crate::platform::macos::appkit::ns_window(window) {
        ns_window.deminiaturize(None);
    }
}

#[cfg(not(target_os = "macos"))]
fn deminiaturize(_: &Window) {}

/// Take `window` off the screen without closing it: its view, terminals and agents go on
/// running (a profile that is not shown, `AppState::switch_profile`). GPUI has no call for it.
#[cfg(target_os = "macos")]
pub fn hide(window: &Window) {
    if let Some(ns_window) = crate::platform::macos::appkit::ns_window(window) {
        ns_window.orderOut(None);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn hide(_: &Window) {}

/// Put a window [`hide`] took away back on the screen, in front but not necessarily keyed.
#[cfg(target_os = "macos")]
pub fn show(window: &Window) {
    if let Some(ns_window) = crate::platform::macos::appkit::ns_window(window) {
        ns_window.orderFront(None);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn show(_: &Window) {}

#[cfg(target_os = "macos")]
fn order_back(window: &Window) {
    if let Some(ns_window) = crate::platform::macos::appkit::ns_window(window) {
        ns_window.orderBack(None);
    }
}

#[cfg(not(target_os = "macos"))]
fn order_back(_: &Window) {}
