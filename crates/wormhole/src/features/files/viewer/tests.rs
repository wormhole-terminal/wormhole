//! The viewer in a real window (GPUI's test platform): a click on a file shows it in the terminal's place,
//! Esc and the other terminals close it, it follows the file on disk without reading what did not change,
//! says when the file is deleted, never follows a link out of the folder, and gives way to a diff.
//!
//! The folders are made in the test's scratch data dir, which the harness removes afterwards.

use std::fs;
use std::path::Path;
use std::rc::Rc;

use gpui_kit::TestAppContext;
use wormhole_core::storage::files::Problem;

use super::{Content, Viewer};
use crate::app_state::OpenWindow;
use crate::features::files::Files;
use crate::features::files::tests::{click, drawn, feature, go, repo, scratch};
use crate::features::git::changes::Changes;
use crate::window::test_harness::{draw, drive, isolate_in, press, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn covered(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    read_cx(cx, window, |ws, cx| ws.is_covered(cx))
}

/// What the viewer says it holds, or `None` when nothing is shown.
fn shown(cx: &mut TestAppContext, window: &OpenWindow) -> Option<String> {
    read_cx(cx, window, |ws, cx| feature(ws).read(cx).viewer.as_ref().map(Viewer::describe))
}

/// The text's lines, to tell a read that happened from one that did not.
fn lines_of(cx: &mut TestAppContext, window: &OpenWindow) -> Option<Rc<super::lines::Lines>> {
    read_cx(cx, window, |ws, cx| match &feature(ws).read(cx).viewer.as_ref()?.content {
        Content::Text(text) => Some(text.lines.clone()),
        _ => None,
    })
}

/// The viewer looks at its file now, as the tick does.
fn look(cx: &mut TestAppContext, window: &OpenWindow) {
    drive(cx, window, |ws, _, cx| {
        feature(ws).update(cx, |files: &mut Files, cx| files.poll_viewer(true, cx))
    });
    cx.run_until_parked();
}

fn open(cx: &mut TestAppContext, window: &OpenWindow, rel: &str) {
    let rel = rel.to_string();
    drive(cx, window, |ws, _, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| files.show_file(session, Path::new(&rel), fcx));
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_click_on_a_file_shows_it_in_the_terminals_place_and_esc_closes_it(cx: &mut TestAppContext) {
    if isolate("a_click_on_a_file_shows_it_in_the_terminals_place_and_esc_closes_it") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    assert!(!covered(cx, &window));

    click(cx, &window, "README.md");
    cx.run_until_parked();
    assert!(covered(cx, &window), "the viewer covers the terminal, as a diff does");
    assert!(
        shown(cx, &window).is_some_and(|text| text.starts_with("README.md (text 1 lines, 5 bytes")),
        "{:?}",
        shown(cx, &window)
    );
    assert!(drawn(cx, &window, "file-viewer"), "the card is drawn");
    let (selected, folder_rows) = read_cx(cx, &window, |ws, cx| {
        let files = feature(ws);
        let of = files.read(cx).of.values().next().expect("looked at").selected.clone();
        (of, ())
    });
    assert_eq!(selected.as_deref(), Some(Path::new("README.md")), "the file's row is selected");
    let () = folder_rows;

    press(cx, &window, "escape");
    assert!(!covered(cx, &window), "Esc closed it");
    assert_eq!(shown(cx, &window), None);
    draw(cx, &window);
}

#[gpui_kit::test]
fn another_file_replaces_the_one_shown_and_a_folder_click_leaves_it_alone(cx: &mut TestAppContext) {
    if isolate("another_file_replaces_the_one_shown_and_a_folder_click_leaves_it_alone") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    click(cx, &window, "README.md");
    cx.run_until_parked();
    click(cx, &window, "src");
    cx.run_until_parked();
    assert!(
        shown(cx, &window).is_some_and(|text| text.starts_with("README.md")),
        "a folder opens in the tree"
    );
    click(cx, &window, "src/main.rs");
    cx.run_until_parked();
    assert!(shown(cx, &window).is_some_and(|text| text.starts_with("src/main.rs")), "one viewer: replaced");
    draw(cx, &window);
}

#[gpui_kit::test]
fn the_file_is_read_again_only_when_it_changed_and_deletion_is_told(cx: &mut TestAppContext) {
    if isolate("the_file_is_read_again_only_when_it_changed_and_deletion_is_told") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "README.md");
    let first = lines_of(cx, &window).expect("text");

    look(cx, &window);
    look(cx, &window);
    assert!(Rc::ptr_eq(&first, &lines_of(cx, &window).unwrap()), "nothing changed: nothing was read");

    fs::write(repo.join("README.md"), b"hello\nagain\n").unwrap();
    look(cx, &window);
    let second = lines_of(cx, &window).expect("text");
    assert!(!Rc::ptr_eq(&first, &second), "it changed: it was read");
    assert_eq!(second.len(), 2);

    fs::remove_file(repo.join("README.md")).unwrap();
    look(cx, &window);
    assert!(
        shown(cx, &window).is_some_and(|text| text.contains("unavailable: Gone")),
        "a deleted file is told so: {:?}",
        shown(cx, &window)
    );
    draw(cx, &window);

    // It comes back.
    fs::write(repo.join("README.md"), b"back\n").unwrap();
    look(cx, &window);
    assert_eq!(lines_of(cx, &window).map(|lines| lines.get(0).to_string()), Some("back".to_string()));
}

#[gpui_kit::test]
fn the_window_tick_looks_at_the_file(cx: &mut TestAppContext) {
    if isolate("the_window_tick_looks_at_the_file") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "README.md");
    fs::write(repo.join("README.md"), b"one\ntwo\nthree\n").unwrap();
    // The ticks that are a multiple of the period look; two ticks include one.
    for _ in 0..2 {
        drive(cx, &window, |ws, window, cx| ws.tick(window, cx));
    }
    cx.run_until_parked();
    assert_eq!(lines_of(cx, &window).map(|lines| lines.len()), Some(3));
}

#[gpui_kit::test]
fn a_link_out_of_the_folder_is_refused_with_a_message(cx: &mut TestAppContext) {
    if isolate("a_link_out_of_the_folder_is_refused_with_a_message") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    let outside = scratch("outside");
    fs::write(outside.join("secret.txt"), b"secret").unwrap();
    wormhole_core::platform::fs::symlink(outside.join("secret.txt"), repo.join("alias.txt")).unwrap();
    go(cx, &window, &repo);
    open(cx, &window, "alias.txt");
    assert!(
        shown(cx, &window).is_some_and(|text| text.contains(&format!("unavailable: {:?}", Problem::Outside))),
        "{:?}",
        shown(cx, &window)
    );
    assert!(drawn(cx, &window, "file-viewer"));
}

#[gpui_kit::test]
fn binary_files_and_pictures_get_their_own_cards(cx: &mut TestAppContext) {
    if isolate("binary_files_and_pictures_get_their_own_cards") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    fs::write(repo.join("tool.bin"), b"MZ\0\0\0\0data").unwrap();
    // A real 1x1 picture.
    let png: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00,
        0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE,
        0x02, 0xFE, 0xA7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60,
        0x82,
    ];
    fs::write(repo.join("dot.png"), png).unwrap();
    go(cx, &window, &repo);
    open(cx, &window, "tool.bin");
    assert_eq!(shown(cx, &window).as_deref(), Some("tool.bin (binary 10 bytes)"));
    assert!(drawn(cx, &window, "file-viewer"));
    open(cx, &window, "dot.png");
    assert_eq!(shown(cx, &window), Some(format!("dot.png (image {} bytes, shown at 1x1)", png.len())));
    assert!(drawn(cx, &window, "file-viewer"));
}

#[gpui_kit::test]
fn a_text_file_gets_its_colors_after_it_shows(cx: &mut TestAppContext) {
    if isolate("a_text_file_gets_its_colors_after_it_shows") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "src/main.rs");
    cx.executor().run_until_parked();
    assert!(shown(cx, &window).is_some_and(|text| text.ends_with(", colored)")), "{:?}", shown(cx, &window));
    draw(cx, &window);
}

#[gpui_kit::test]
fn the_viewer_and_a_diff_never_cover_the_terminal_together(cx: &mut TestAppContext) {
    if isolate("the_viewer_and_a_diff_never_cover_the_terminal_together") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "README.md");
    assert!(shown(cx, &window).is_some());

    let diff_is_open = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| ws.feature::<Changes>().is_some_and(|c| c.read(cx).diff_open()))
    };
    drive(cx, &window, |ws, _, cx| {
        let session = ws.active_session().id;
        let change = wormhole_core::git::FileChange {
            path: "src/lib.rs".into(),
            original_path: None,
            kind: wormhole_core::git::ChangeKind::Modified,
            added: Some(1),
            removed: Some(0),
            lfs: false,
            submodule: None,
        };
        let spec = wormhole_core::git::DiffSpec { old: "HEAD".into(), new: None };
        ws.feature::<Changes>().expect("in the table").update(cx, |changes, cx| {
            changes.open_diff(session, "/nonexistent".into(), change, spec, Rc::default(), None, cx);
        });
    });
    assert!(diff_is_open(cx), "the diff opened");
    assert_eq!(shown(cx, &window), None, "and the viewer came down");

    open(cx, &window, "README.md");
    assert!(shown(cx, &window).is_some());
    assert!(!diff_is_open(cx), "the viewer opened and the diff came down");
}

#[gpui_kit::test]
fn selecting_another_terminal_closes_the_viewer(cx: &mut TestAppContext) {
    if isolate("selecting_another_terminal_closes_the_viewer") {
        return;
    }
    let window = start(cx, 2);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "README.md");
    assert!(covered(cx, &window));
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert!(!covered(cx, &window), "the viewer belonged to the first terminal");
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    assert!(!covered(cx, &window), "and does not come back");
}

/// How many reads and color jobs were started so far.
fn started(cx: &mut TestAppContext, window: &OpenWindow) -> (u64, u64) {
    read_cx(cx, window, |ws, cx| feature(ws).read(cx).started)
}

#[gpui_kit::test]
fn walking_over_files_starts_one_read_at_a_time_and_the_latest_wins(cx: &mut TestAppContext) {
    if isolate("walking_over_files_starts_one_read_at_a_time_and_the_latest_wins") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    for n in 0..6 {
        fs::write(repo.join(format!("f{n}.txt")), format!("file {n}\n")).unwrap();
    }
    go(cx, &window, &repo);
    let before = started(cx, &window);
    // The arrow keys ask for a file per row, faster than a read comes back.
    drive(cx, &window, |ws, _, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| {
            for n in 0..6 {
                files.show_file(session, Path::new(&format!("f{n}.txt")), fcx);
            }
        });
    });
    cx.run_until_parked();
    let after = started(cx, &window);
    assert_eq!(after.0 - before.0, 2, "the first, then the one wanted when it came back: not six");
    assert!(
        shown(cx, &window).is_some_and(|text| text.starts_with("f5.txt (text 1 lines")),
        "the latest request is what is shown: {:?}",
        shown(cx, &window)
    );
}

#[gpui_kit::test]
fn colors_are_made_one_job_at_a_time_and_not_again_for_a_file_without_a_grammar(cx: &mut TestAppContext) {
    if isolate("colors_are_made_one_job_at_a_time_and_not_again_for_a_file_without_a_grammar") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "src/main.rs");
    let colored = started(cx, &window).1;
    assert!(colored >= 1);
    // Asked for again, with the colors in: nothing starts. A file without a grammar is not retried.
    drive(cx, &window, |ws, _, cx| {
        feature(ws).update(cx, |files, fcx| {
            files.colour_viewer(fcx);
            files.colour_viewer(fcx);
        });
    });
    assert_eq!(started(cx, &window).1, colored);
    open(cx, &window, "README.md");
    let plain = started(cx, &window).1;
    drive(cx, &window, |ws, _, cx| {
        feature(ws).update(cx, |files, fcx| {
            files.colour_viewer(fcx);
            files.colour_viewer(fcx);
        });
    });
    assert_eq!(started(cx, &window).1, plain, "a second look at the same file starts nothing");
}

#[gpui_kit::test]
fn nothing_is_read_or_polled_while_the_feature_is_off(cx: &mut TestAppContext) {
    use wormhole_core::config::{FeatureId, TabId};
    if isolate("nothing_is_read_or_polled_while_the_feature_is_off") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    open(cx, &window, "README.md");
    let first = lines_of(cx, &window).expect("text");
    fs::write(repo.join("README.md"), b"changed\nlines\n").unwrap();
    let tick = |cx: &mut TestAppContext, on: bool, ticks: u64| {
        drive(cx, &window, |ws, _, cx| {
            let mut facts = ws.facts(cx);
            facts.tab = TabId::Files;
            facts.ticks = ticks;
            facts.features = facts.features.with(FeatureId::Files, on);
            feature(ws).update(cx, |files, fcx| files.tick_viewer(&facts, fcx));
        });
    };
    let reads = started(cx, &window);
    tick(cx, false, 2);
    tick(cx, false, 4);
    assert_eq!(started(cx, &window), reads, "no read while the flag is off");
    assert!(Rc::ptr_eq(&first, &lines_of(cx, &window).unwrap()));
    tick(cx, true, 4);
    assert!(!Rc::ptr_eq(&first, &lines_of(cx, &window).unwrap()), "and the same tick reads when it is on");
}

#[gpui_kit::test]
fn a_picture_is_dropped_from_the_atlas_when_it_is_replaced_or_the_viewer_goes(cx: &mut TestAppContext) {
    if isolate("a_picture_is_dropped_from_the_atlas_when_it_is_replaced_or_the_viewer_goes") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    write_png(&repo.join("a.png"), 8, 8);
    write_png(&repo.join("b.png"), 8, 8);
    go(cx, &window, &repo);
    let image_of = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| match &feature(ws).read(cx).viewer.as_ref()?.content {
            Content::Image { image: Ok(image), .. } => Some(image.clone()),
            _ => None,
        })
    };
    let in_atlas = |cx: &mut TestAppContext, image: &std::sync::Arc<gpui_kit::RenderImage>| {
        cx.update(|cx| window.handle.update(cx, |_, window, _| window.has_image_atlas_entry(image)).unwrap())
    };

    open(cx, &window, "a.png");
    draw(cx, &window);
    let first = image_of(cx).expect("the picture was decoded");
    assert!(in_atlas(cx, &first), "drawing put it in the atlas");

    // Another file replaces it: the first picture's tile goes.
    open(cx, &window, "b.png");
    cx.run_until_parked();
    draw(cx, &window);
    assert!(!in_atlas(cx, &first), "replaced: dropped from the atlas");
    let second = image_of(cx).expect("the second picture");
    assert!(in_atlas(cx, &second));

    // Esc closes the viewer: the second one goes too.
    press(cx, &window, "escape");
    cx.run_until_parked();
    assert!(!in_atlas(cx, &second), "closed: dropped from the atlas");
}

/// A `width` x `height` PNG of one color.
fn write_png(path: &Path, width: u32, height: u32) {
    let buffer = image::RgbaImage::from_pixel(width, height, image::Rgba([200, 40, 40, 255]));
    buffer.save_with_format(path, image::ImageFormat::Png).expect("writes a PNG");
}

#[gpui_kit::test]
fn one_very_long_line_opens_and_draws_without_shaping_all_of_it(cx: &mut TestAppContext) {
    if isolate("one_very_long_line_opens_and_draws_without_shaping_all_of_it") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    // A minified bundle: one line of 1.5 million characters, with something to color.
    let long = format!("var a = [{}];", "1,".repeat(750_000));
    fs::write(repo.join("bundle.js"), long).unwrap();
    go(cx, &window, &repo);
    open(cx, &window, "bundle.js");
    cx.run_until_parked();
    let started = std::time::Instant::now();
    assert!(drawn(cx, &window, "file-viewer"));
    draw(cx, &window);
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    assert!(shown(cx, &window).is_some_and(|text| text.contains("text 1 lines")), "{:?}", shown(cx, &window));
}

#[gpui_kit::test]
fn a_picture_that_declares_an_absurd_size_is_refused_not_decoded(cx: &mut TestAppContext) {
    if isolate("a_picture_that_declares_an_absurd_size_is_refused_not_decoded") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    // A few bytes of PNG header that declare 100,000 x 100,000 pixels: no pixel data follows.
    fs::write(repo.join("bomb.png"), crate::platform::image::crafted::png_declaring(100_000, 100_000))
        .unwrap();
    go(cx, &window, &repo);
    open(cx, &window, "bomb.png");
    assert!(
        shown(cx, &window)
            .is_some_and(|text| text.contains("too large") || text.contains("can't be decoded")),
        "{:?}",
        shown(cx, &window)
    );
    assert!(drawn(cx, &window, "file-viewer"));
}
