//! Settings → Motion is behind a feature flag: it is in the nav, after Fonts, and its choices are
//! found by the search only while the flag is on. The flag is a switch of the person's own
//! `feature_flags`, so it is turned off in the file's way: with an entry.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::TestAppContext;
use wormhole_core::config::settings_index::{self, Query};

use super::{Section, open_settings};
use crate::settings::{settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

fn listed(cx: &mut TestAppContext) -> Vec<Section> {
    cx.update(|cx| Section::visible(settings(cx)).collect())
}

fn found(cx: &mut TestAppContext, word: &str) -> Vec<&'static str> {
    cx.update(|cx| {
        settings_index::search_visible(&Query::new(word), settings(cx))
            .into_iter()
            .map(|hit| hit.entry.id)
            .collect()
    })
}

#[gpui_kit::test]
fn the_motion_page_and_its_choices_show_only_while_its_flag_is_on(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_motion_page_and_its_choices_show_only_while_its_flag_is_on") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Motion), cx));
    cx.run_until_parked();

    // A development build has every flag on until it is switched off.
    let on = listed(cx);
    let fonts = on.iter().position(|s| *s == Section::Fonts).expect("Fonts is listed");
    assert_eq!(on.get(fonts + 1), Some(&Section::Motion), "Motion follows Fonts in the Look group");
    assert!(found(cx, "tuck").contains(&"motion-fold"), "the search finds a choice by its option's name");

    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert("motion".into(), false);
        })
    });
    assert!(!listed(cx).contains(&Section::Motion), "off: not in the nav");
    assert!(found(cx, "fold").iter().all(|id| !id.starts_with("motion-")), "off: not in the search");
    assert!(listed(cx).contains(&Section::Fonts), "and nothing else went with it");
}
