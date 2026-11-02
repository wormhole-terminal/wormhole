//! The profile's own files followed through events: which paths matter, and, end to end through the
//! system's real file events in the test's scratch data dir, that a `config.json` written from outside
//! applies with no tick.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::platform::watch::Batch;
use wormhole_core::storage::paths::{DataRoot, ProfileId};

use super::{CONFIG, FONTS, OwnFiles, Roots};
use crate::settings::{SettingsStore, settings};
use crate::window::test_harness::{isolate_in, start};

fn roots() -> Roots {
    let paths = DataRoot::new("/data").profile(&ProfileId::default_profile());
    Roots::of(&paths, Path::new("/real/data"))
}

fn batch(paths: &[&str], rescan: bool) -> Batch {
    Batch { paths: paths.iter().map(PathBuf::from).collect(), rescan }
}

#[::core::prelude::v1::test]
fn only_the_config_the_themes_and_the_fonts_matter() {
    let roots = roots();
    assert_eq!(roots.classify_batch(&batch(&["/real/data/config.json"], false)), CONFIG);
    assert_eq!(roots.classify_batch(&batch(&["/real/data/themes/Mine.json"], false)), CONFIG);
    assert_eq!(roots.classify_batch(&batch(&["/real/data/fonts/Mono.ttf"], false)), FONTS);
    assert_eq!(
        roots.classify_batch(&batch(&["/real/data/themes/a.json", "/real/data/fonts/b.otf"], false)),
        CONFIG | FONTS
    );
    // The database, the logs and a file that only starts like a name are not ours.
    for other in ["/real/data/wormhole.db", "/real/data/logs/app.log", "/real/data/config.json.tmp"] {
        assert_eq!(roots.classify_batch(&batch(&[other], false)), 0, "{other}");
    }
    assert_eq!(roots.classify_batch(&batch(&["/real/data/fontsy/x"], false)), 0);
}

#[::core::prelude::v1::test]
fn lost_events_ask_for_everything() {
    let roots = roots();
    assert_eq!(roots.classify_batch(&batch(&[], true)), CONFIG | FONTS);
    assert_eq!(roots.classify_batch(&batch(&["/real/data/logs"], true)), CONFIG | FONTS);
}

fn wait_for(cx: &mut TestAppContext, what: &str, mut done: impl FnMut(&mut TestAppContext) -> bool) {
    for _ in 0..200 {
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
        cx.executor().advance_clock(Duration::from_millis(50));
        cx.run_until_parked();
    }
    panic!("never saw: {what}");
}

#[gpui_kit::test]
fn a_config_written_from_outside_applies_without_a_tick(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_config_written_from_outside_applies_without_a_tick") {
        return;
    }
    let _window = start(cx, 1);
    let path = SettingsStore::config_path();
    cx.update(|cx| {
        cx.set_global(SettingsStore::load());
        crate::fonts::init(cx);
        OwnFiles::follow(cx);
    });
    wait_for(cx, "the profile folder is followed", |cx| cx.update(|cx| OwnFiles::live(cx)));
    let before = cx.update(|cx| settings(cx).font_size);

    // What an editor or `devctl set` does: replace the file. No tick runs anywhere in this test.
    let text = format!("{{ \"font_size\": {} }}\n", before + 3.0);
    let staged = path.with_extension("tmp");
    std::fs::write(&staged, text).expect("write");
    std::fs::rename(&staged, &path).expect("replace");
    wait_for(cx, "the new font size", |cx| cx.update(|cx| settings(cx).font_size) == before + 3.0);
}
