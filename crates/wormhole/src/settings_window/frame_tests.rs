//! What a Settings frame costs in allocations, page by page, with the words of the window in the
//! catalogs. A word without variables is an array index and costs none; a sentence with variables is
//! formatted by Fluent (a few allocations more than a `format!`, five for one with a number), and a page
//! that does so for every row of every frame would show here.
//!
//! [`BEFORE`] is what a third draw of each page allocated while the words were still written in Rust
//! (measured on the commit before the Settings window's words moved); a frame may make a hundredth more.
//! The pages with sentences made a few more (2 to 14 of about 10,000); the search made 14% fewer, since
//! its words are lower-cased once per language and no longer for every keystroke.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, TestAppContext};
use wormhole_core::config::settings_index::Query;

use super::{Section, current_window, open_settings};
use crate::perf::allocations::thread_counts;
use crate::window::test_harness::{isolate_in, start};

/// Allocations a third draw of each page made before the move (the first two build what a window keeps),
/// and of the results page of a search for a word found in several pages.
const BEFORE: [(&str, u64); 14] = [
    ("Profile", 9_847),
    ("Appearance", 14_383),
    ("Fonts", 7_247),
    ("Terminal", 12_521),
    ("Sessions", 13_750),
    ("Git", 7_745),
    ("Agents", 27_044),
    ("Notifications", 10_353),
    ("MenuBar", 7_852),
    ("Updates", 6_621),
    ("Advanced", 7_318),
    ("Projects", 12_190),
    ("FeatureFlags", 6_742),
    ("search", 13_259),
];

fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx)))
        .expect("the settings window is open");
}

#[gpui_kit::test]
fn a_settings_frame_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_settings_frame_stays_within_its_allocation_budget") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Profile), cx));
    cx.run_until_parked();
    let (handle, page) = cx.update(|cx| current_window(cx)).expect("the settings window opened");
    let mut measured = Vec::new();
    for section in Section::ALL {
        cx.update(|cx| page.update(cx, |page, cx| page.show_section(section, cx)));
        cx.run_until_parked();
        draw(cx, handle);
        draw(cx, handle);
        let before = thread_counts();
        draw(cx, handle);
        measured.push((format!("{section:?}"), thread_counts().since(before)));
    }
    // A search: the results of a word found in several pages, over the pages' rows.
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, cx| {
                    page.search.update(cx, |search, cx| search.set_value("line".to_string(), window, cx));
                    page.show_results = !Query::new("line").is_empty();
                    cx.notify();
                })
            })
            .expect("the window is open");
    });
    cx.run_until_parked();
    draw(cx, handle);
    draw(cx, handle);
    let before = thread_counts();
    draw(cx, handle);
    measured.push(("search".to_string(), thread_counts().since(before)));

    let table: Vec<String> = measured
        .iter()
        .map(|(name, drawn)| format!("{name}: {} allocations, {} bytes", drawn.allocations, drawn.bytes))
        .collect();
    let over: Vec<String> = measured
        .iter()
        .zip(BEFORE)
        .filter(|((_, drawn), (_, before))| drawn.allocations > before + before / 100)
        .map(|((name, drawn), (_, before))| {
            format!(
                "{name}: {} (before the move {before}, budget {})",
                drawn.allocations,
                before + before / 100
            )
        })
        .collect();
    assert!(over.is_empty(), "frames over their budget:\n{}\nall:\n{}", over.join("\n"), table.join("\n"));
}
