//! What a Settings frame costs in allocations, page by page, with the words of the window in the
//! catalogs. A word without variables is an array index and costs none; a sentence with variables is
//! formatted by Fluent (a few allocations more than a `format!`, five for one with a number), and a page
//! that does so for every row of every frame would show here.
//!
//! [`BUDGET`] is what a third draw of each page allocates today plus [`SLACK`], a fixed margin for small
//! drift. The reference is what a third draw allocated while the words were still written in Rust
//! (measured on the commit before the Settings window's words moved): Profile 9,847, Appearance 14,383,
//! Fonts 7,247, Terminal 12,521, Sessions 13,750, Git 7,745, Agents 27,044, Notifications 10,353,
//! MenuBar 7,852, Updates 6,621, Advanced 7,318, Projects 12,190, FeatureFlags 6,742, search 13,259.
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

/// Allocations a frame may make over what the pages made when this budget was set.
const SLACK: u64 = 20;

/// Allocations a third draw of each page made when the budget was set (the first two build what a window
/// keeps), and of the results page of a search for a word found in several pages; a frame may make
/// [`SLACK`] more.
const BUDGET: [(&str, u64); 14] = [
    ("Profile", 9849),
    ("Appearance", 14388),
    ("Fonts", 7259),
    ("Terminal", 12535),
    ("Sessions", 13752),
    ("Git", 7749),
    ("Agents", 27055),
    ("Notifications", 10355),
    ("MenuBar", 7852),
    ("Updates", 6621),
    ("Advanced", 7318),
    ("Projects", 12196),
    ("FeatureFlags", 6742),
    ("search", 11356),
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
        .zip(BUDGET)
        .filter(|((_, drawn), (_, budget))| drawn.allocations > budget + SLACK)
        .map(|((name, drawn), (_, budget))| {
            format!("{name}: {} (budget {})", drawn.allocations, budget + SLACK)
        })
        .collect();
    assert!(over.is_empty(), "frames over their budget:\n{}\nall:\n{}", over.join("\n"), table.join("\n"));
}
