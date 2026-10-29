//! What the Studio draws, counted: the gallery and the token list build the part of their page in
//! view, not all of it, and a wheel tick redraws the view it is over and no other. Counts, not
//! timings, so they hold on any machine and in a debug build (`../repo-docs/docs/performance.md`).
//! A headless GPUI with the window at its usual size; needs `devctl`'s test support.

use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions, px, size,
};

use super::draft::groups;
use super::gallery;
use super::window_tests::{opened, themes};
use super::{StudioWindow, token_list};
use crate::settings::SettingsStore;

fn open(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<StudioWindow>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        let bounds = Bounds::centered(None, size(px(1400.0), px(900.0)), cx);
        let options =
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() };
        let doc = opened(&themes(), cx);
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| StudioWindow::with_themes(themes(), doc, window, cx))
        })
        .expect("opens the window")
    })
}

fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update(|cx| {
        handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)).expect("the window is open");
    });
}

/// Draw until the page has settled: a piece that has been built and measured moves what is below
/// it, which asks for another frame (the test platform has no display link to deliver it).
fn settle(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    for _ in 0..8 {
        draw(cx, handle);
        let asked = cx.update(|cx| {
            handle.update(cx, |_, window, cx| window.simulate_next_frame(cx)).expect("the window")
        });
        if asked == 0 {
            return;
        }
    }
    draw(cx, handle);
}

/// What each view has done so far.
struct Seen {
    gallery_renders: usize,
    pieces: usize,
    list: token_list::Counts,
    editor_renders: usize,
}

fn seen(cx: &mut TestAppContext, studio: &Entity<StudioWindow>) -> Seen {
    cx.update(|cx| {
        let studio = studio.read(cx);
        let gallery = studio.gallery.read(cx);
        Seen {
            gallery_renders: gallery.counts.renders,
            pieces: gallery.counts.pieces.get(),
            list: studio.list.read(cx).counts,
            editor_renders: studio.editor.read(cx).renders,
        }
    })
}

/// A draw that nothing asked for draws nothing again: every view's last frame is reused.
#[gpui_kit::test]
fn a_window_that_did_not_change_is_not_built_again(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    settle(cx, handle);
    let before = seen(cx, &studio);
    draw(cx, handle);
    draw(cx, handle);
    let after = seen(cx, &studio);
    assert_eq!(
        (after.gallery_renders, after.pieces, after.list, after.editor_renders),
        (before.gallery_renders, before.pieces, before.list, before.editor_renders)
    );
}

/// The page is twenty specimens (thirty-odd pieces); a frame at the top builds the ones in view,
/// not all of them as it did when it was one scrolling div.
#[gpui_kit::test]
fn the_gallery_builds_the_pieces_in_view_and_not_the_page(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    settle(cx, handle);
    // One frame of the settled page at the top.
    let gallery = cx.update(|cx| studio.read(cx).gallery.clone());
    cx.update(|cx| {
        gallery.update(cx, |gallery, cx| {
            gallery.counts.pieces.set(0);
            cx.notify();
        })
    });
    draw(cx, handle);
    let built = seen(cx, &studio).pieces;
    let page = gallery::piece_count();
    assert!(built > 0, "nothing was built");
    assert!(built * 2 <= page, "built {built} of the page's {page} pieces in a frame");
}

/// The token list has 107 tokens in 20 bands; with every band open a frame builds the rows in view.
#[gpui_kit::test]
fn the_token_list_builds_the_rows_in_view_and_not_the_list(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    let list = cx.update(|cx| studio.read(cx).list.clone());
    cx.update(|cx| {
        list.update(cx, |list, cx| {
            for group in groups() {
                list.open_group(group.name, cx);
            }
        });
    });
    draw(cx, handle);
    draw(cx, handle);
    let rows = cx.update(|cx| list.read(cx).row_count());
    let built = seen(cx, &studio).list.rows_built;
    assert!(rows > 100, "{rows} rows");
    // The list lays its rows out, then draws them: a frame builds what is in view and its overdraw
    // twice, against every row twice when all of them are built.
    assert!(built > 0 && built * 4 <= rows * 3, "built {built} of {rows} rows");
}

/// A wheel tick over the gallery redraws the gallery, not the token list or the editor (it used to
/// notify the whole window, which built all three again).
#[gpui_kit::test]
fn scrolling_the_gallery_redraws_the_gallery_alone(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    settle(cx, handle);
    let gallery = cx.update(|cx| studio.read(cx).gallery.clone());
    let tick = |cx: &mut TestAppContext| {
        cx.update(|cx| gallery.update(cx, |gallery, cx| gallery.scroll_by(300.0, cx)));
        settle(cx, handle);
    };
    tick(cx);
    let before = seen(cx, &studio);
    for _ in 0..3 {
        tick(cx);
    }
    let after = seen(cx, &studio);
    assert!(after.gallery_renders >= before.gallery_renders + 3, "the gallery is drawn for each tick");
    assert_eq!(after.list, before.list, "the token list is not");
    assert_eq!(after.editor_renders, before.editor_renders, "the editor is not");
}

/// A new selection makes the token list's rows again, once, and the editor shows the token.
#[gpui_kit::test]
fn the_token_list_and_the_editor_are_drawn_when_the_selection_changes(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    draw(cx, handle);
    draw(cx, handle);
    let before = seen(cx, &studio);
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                studio.update(cx, |studio, cx| studio.select_token("text.primary", window, cx))
            })
            .expect("the window is open");
    });
    draw(cx, handle);
    let after = seen(cx, &studio);
    assert_eq!(after.list.syncs, before.list.syncs + 1, "the rows are made again, once");
    assert_eq!(after.editor_renders, before.editor_renders + 1);
}

/// Every piece of every specimen builds: the page scrolled from top to bottom.
#[gpui_kit::test]
fn the_whole_page_builds_piece_by_piece(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    settle(cx, handle);
    let gallery = cx.update(|cx| studio.read(cx).gallery.clone());
    for _ in 0..40 {
        cx.update(|cx| gallery.update(cx, |gallery, cx| gallery.scroll_by(400.0, cx)));
        settle(cx, handle);
    }
    let built = seen(cx, &studio).pieces;
    assert!(built >= gallery::piece_count(), "only {built} pieces were built");
}

/// Switching Light | Dark makes the list's rows once, however often it is drawn after.
#[gpui_kit::test]
fn the_light_dark_switch_makes_the_rows_once(cx: &mut TestAppContext) {
    use wormhole_core::theme::tokens::Appearance;
    let (handle, studio) = open(cx);
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                studio.update(cx, |s, cx| s.add_appearance(Appearance::Light, window, cx))
            })
            .expect("the window is open")
    });
    settle(cx, handle);
    let before = seen(cx, &studio);
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| studio.update(cx, |s, cx| s.set_shown("dark", window, cx)))
            .expect("the window is open")
    });
    settle(cx, handle);
    draw(cx, handle);
    let after = seen(cx, &studio);
    assert_eq!(after.list.syncs - before.list.syncs, 1, "the rows were made once for the switch");
}
