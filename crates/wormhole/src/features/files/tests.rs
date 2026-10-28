//! What the Files tab does through the window: which folder it is of and how that follows the terminal,
//! the path menu's choices and pins, what a click and a double-click do, and what it never does (enter a
//! link, read while hidden).
//!
//! The folders are made in the test's scratch data dir, which the harness removes afterwards.

use std::fs;
use std::path::{Path, PathBuf};

use gpui_kit::{TestAppContext, VisualTestContext};
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::platform::open_file::Launch;
use wormhole_core::view::file_tree::{Row, RowKind};

use super::read::Read;
use super::{Files, open};
use crate::app_state::OpenWindow;
use crate::window::Workspace;
use crate::window::test_harness::{draw, drive, isolate_in, press, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A folder of the test's scratch dir.
fn scratch(name: &str) -> PathBuf {
    let data = std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir");
    let dir = PathBuf::from(data).join(name);
    fs::create_dir_all(&dir).expect("makes the scratch folder");
    dir
}

/// A repository (a `.git` is enough to be one) with `src/main.rs`, `docs/` and a `README.md`.
fn repo(name: &str) -> PathBuf {
    let root = scratch(name);
    for dir in [".git", "src", "docs"] {
        fs::create_dir_all(root.join(dir)).expect("makes a folder");
    }
    fs::write(root.join("README.md"), b"hello").expect("writes");
    fs::write(root.join("src/main.rs"), b"fn main() {}").expect("writes");
    fs::write(root.join("src/lib.rs"), b"").expect("writes");
    root
}

fn feature(ws: &Workspace) -> gpui_kit::Entity<Files> {
    ws.feature::<Files>().expect("the window has the files feature")
}

/// The terminal in front is in `dir`, and the Files tab is on show and has read it.
fn go(cx: &mut TestAppContext, window: &OpenWindow, dir: &Path) {
    let dir = dir.to_path_buf();
    drive(cx, window, |ws, _, cx| {
        ws.active_terminal().update(cx, |terminal, cx| {
            terminal.cwd = Some(dir);
            cx.notify();
        });
        ws.sync_summaries(cx);
        ws.show_tab(TabId::Files, cx);
    });
    // The draw follows the folder and asks for the read; the read lands a moment later.
    draw(cx, window);
    cx.run_until_parked();
    draw(cx, window);
}

/// The tab as text: its first line names the folder, the others are the rows.
fn tree(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    let text = drive(cx, window, |ws, _, cx| super::devctl::describe(ws, cx).expect("describes"));
    text.lines().map(str::to_string).collect()
}

/// Just the labels of the rows, as `describe` marks them (`v` open, `>` closed), without indentation.
fn rows(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    tree(cx, window).into_iter().skip(1).map(|line| line.trim_start_matches([' ', '*']).to_string()).collect()
}

fn root(cx: &mut TestAppContext, window: &OpenWindow) -> Option<PathBuf> {
    read_cx(cx, window, |ws, cx| {
        let session = ws.active_session().id;
        feature(ws).read(cx).of.get(&session).and_then(|of| of.root.clone())
    })
}

fn row(cx: &mut TestAppContext, window: &OpenWindow, rel: &str) -> Row {
    read_cx(cx, window, |ws, cx| {
        let session = ws.active_session().id;
        let files = feature(ws);
        let files = files.read(cx);
        let of = files.of.get(&session).expect("looked at");
        of.tree
            .rows()
            .into_iter()
            .find(|row| row.path == Path::new(rel))
            .unwrap_or_else(|| panic!("no row {rel}"))
    })
}

/// A click on the row, as the list's handler runs it.
fn click(cx: &mut TestAppContext, window: &OpenWindow, rel: &str, double: bool) {
    let target = row(cx, window, rel);
    drive(cx, window, |ws, _, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| files.click(session, &target, double, fcx));
    });
}

fn drawn(cx: &mut TestAppContext, window: &OpenWindow, name: &str) -> bool {
    draw(cx, window);
    let name: &'static str = Box::leak(name.to_string().into_boxed_str());
    VisualTestContext::from_window(window.handle, cx).debug_bounds(name).is_some()
}

#[gpui_kit::test]
fn the_tree_is_of_the_repository_root_and_follows_the_terminal(cx: &mut TestAppContext) {
    if isolate("the_tree_is_of_the_repository_root_and_follows_the_terminal") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo.join("src"));
    assert_eq!(root(cx, &window), Some(repo.clone()), "in a repository: its root, not the folder");
    assert_eq!(
        rows(cx, &window),
        ["> .git", "> docs", "> src", "README.md"],
        "everything, folders first, closed"
    );
    assert!(drawn(cx, &window, "files-path") && drawn(cx, &window, "files-row-src"));

    // `cd` to a folder that is no repository: that folder itself.
    let plain = scratch("plain");
    fs::write(plain.join("b10.txt"), b"").unwrap();
    fs::write(plain.join("b2.txt"), b"").unwrap();
    go(cx, &window, &plain);
    assert_eq!(root(cx, &window), Some(plain.clone()));
    assert_eq!(rows(cx, &window), ["b2.txt", "b10.txt"], "natural order: 2 before 10");
    assert_eq!(tree(cx, &window)[0], format!("files: {} (following) pinned=-", plain.display()));
}

#[gpui_kit::test]
fn choosing_a_parent_holds_the_tree_there_until_the_terminal_is_followed_again(cx: &mut TestAppContext) {
    if isolate("choosing_a_parent_holds_the_tree_there_until_the_terminal_is_followed_again") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    let parent = repo.parent().unwrap().to_path_buf();
    drive(cx, &window, |ws, _, cx| super::devctl::root(ws, &parent, cx).expect("chooses"));
    draw(cx, &window);
    cx.run_until_parked();
    assert_eq!(root(cx, &window), Some(parent.clone()));
    assert!(tree(cx, &window)[0].contains("(chosen)"));

    // The terminal moving does not take the tree along while a folder is chosen ...
    let plain = scratch("plain");
    go(cx, &window, &plain);
    assert_eq!(root(cx, &window), Some(parent), "held");
    // ... until Follow Terminal.
    drive(cx, &window, |ws, _, cx| super::devctl::follow(ws, cx).expect("follows"));
    draw(cx, &window);
    cx.run_until_parked();
    assert_eq!(root(cx, &window), Some(plain));
    assert!(tree(cx, &window)[0].contains("(following)"));
}

#[gpui_kit::test]
fn the_path_menu_lists_the_parents_the_pins_and_the_commands(cx: &mut TestAppContext) {
    if isolate("the_path_menu_lists_the_parents_the_pins_and_the_commands") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    let menu = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, _, cx| {
            let session = ws.active_session().id;
            feature(ws).update(cx, |files, fcx| files.path_menu_for(session, fcx)).expect("a menu").outline()
        })
    };
    let before = menu(cx);
    assert!(before[0].ends_with("project [x]"), "the folder, checked: {before:?}");
    assert!(before.contains(&"---".to_string()));
    assert!(before.contains(&"- Pin “project”".to_string()), "{before:?}");
    assert!(before.contains(&"- Choose Folder…".to_string()));
    assert!(!before.iter().any(|line| line.contains("Follow Terminal")), "nothing chosen yet");

    // Pinning lands in the settings (the config.json of the scratch data dir), and the menu offers to unpin.
    drive(cx, &window, |ws, _, cx| {
        super::devctl::pin(ws, &repo, cx).expect("pins");
    });
    let pinned = cx.update(|cx| crate::settings::settings(cx).files_pinned.clone());
    assert_eq!(pinned, [repo.to_string_lossy().to_string()]);
    let after = menu(cx);
    assert!(after.contains(&"- Unpin “project”".to_string()), "{after:?}");
    assert_eq!(
        after.iter().filter(|line| line.ends_with("- project")).count(),
        1,
        "the pin is listed: {after:?}"
    );

    // A chosen folder adds Follow Terminal.
    let parent = repo.parent().unwrap().to_path_buf();
    drive(cx, &window, |ws, _, cx| super::devctl::root(ws, &parent, cx).expect("chooses"));
    assert!(menu(cx).iter().any(|line| line == "- Follow Terminal"));
}

#[gpui_kit::test]
fn a_pin_is_written_to_the_config_file(cx: &mut TestAppContext) {
    if isolate("a_pin_is_written_to_the_config_file") {
        return;
    }
    let _window = start(cx, 1);
    let folder = scratch("pinned");
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.set_pinned(&folder, true);
        })
    });
    cx.run_until_parked();
    let file = crate::settings::SettingsStore::config_path();
    let text = fs::read_to_string(file).expect("config.json was written");
    assert!(text.contains("files_pinned") && text.contains("pinned"), "{text}");
}

#[gpui_kit::test]
fn a_folder_opens_on_a_click_and_a_link_to_a_folder_never_does(cx: &mut TestAppContext) {
    if isolate("a_folder_opens_on_a_click_and_a_link_to_a_folder_never_does") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    let outside = scratch("outside");
    fs::write(outside.join("secret.txt"), b"").unwrap();
    wormhole_core::platform::fs::symlink(&outside, repo.join("escape")).unwrap();
    go(cx, &window, &repo);
    assert_eq!(rows(cx, &window), ["> .git", "> docs", "> src", "~ escape", "README.md"].map(String::from));

    click(cx, &window, "src", false);
    cx.run_until_parked();
    assert_eq!(
        rows(cx, &window),
        ["> .git", "> docs", "v src", "lib.rs", "main.rs", "~ escape", "README.md"].map(String::from)
    );
    click(cx, &window, "src", false);
    assert_eq!(rows(cx, &window).len(), 5, "closed again");

    click(cx, &window, "escape", false);
    click(cx, &window, "escape", true);
    cx.run_until_parked();
    assert!(!rows(cx, &window).iter().any(|line| line.contains("secret")), "the link was not entered");
    assert!(matches!(row(cx, &window, "escape").kind, RowKind::Link));
}

#[gpui_kit::test]
fn a_double_click_on_a_file_hands_it_to_the_app_and_a_link_is_refused(cx: &mut TestAppContext) {
    if isolate("a_double_click_on_a_file_hands_it_to_the_app_and_a_link_is_refused") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    let outside = scratch("outside");
    fs::write(outside.join("passwd"), b"").unwrap();
    wormhole_core::platform::fs::symlink(outside.join("passwd"), repo.join("passwd-link")).unwrap();
    go(cx, &window, &repo);
    open::record::take();

    click(cx, &window, "README.md", false);
    assert!(open::record::take().is_empty(), "one click selects");
    click(cx, &window, "README.md", true);
    let launched = open::record::take();
    assert_eq!(launched.len(), 1, "{launched:?}");
    let real = fs::canonicalize(repo.join("README.md")).unwrap();
    assert!(
        matches!(&launched[0].1, Launch::Open { path } | Launch::Reveal { path } if *path == real),
        "{launched:?}"
    );

    click(cx, &window, "passwd-link", true);
    assert!(open::record::take().is_empty(), "a link may lead anywhere: it is not opened");
}

#[gpui_kit::test]
fn the_row_menu_offers_open_reveal_copy_and_pins_only_folders(cx: &mut TestAppContext) {
    if isolate("the_row_menu_offers_open_reveal_copy_and_pins_only_folders") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    wormhole_core::platform::fs::symlink(repo.join("README.md"), repo.join("alias")).unwrap();
    go(cx, &window, &repo);
    let menu_of = |cx: &mut TestAppContext, rel: &str| {
        let target = row(cx, &window, rel);
        let facts = super::menus::RowMenu {
            root: repo.clone(),
            apps: Vec::new(),
            chosen: String::new(),
            pinned: false,
        };
        super::menus::row_menu(&target, facts, &gpui_kit::WeakEntity::new_invalid()).outline()
    };
    assert_eq!(menu_of(cx, "src"), ["> Open in", "- Reveal in Finder", "- Copy Path", "---", "- Pin Folder"]);
    assert_eq!(menu_of(cx, "README.md"), ["> Open in", "- Reveal in Finder", "- Copy Path"]);
    assert_eq!(
        menu_of(cx, "alias"),
        ["> Open in [off]", "- Reveal in Finder", "- Copy Path"],
        "a link is not opened"
    );
}

#[gpui_kit::test]
fn the_tab_reads_nothing_while_it_is_hidden_and_notes_a_vanished_folder(cx: &mut TestAppContext) {
    if isolate("the_tab_reads_nothing_while_it_is_hidden_and_notes_a_vanished_folder") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    click(cx, &window, "docs", false);
    cx.run_until_parked();
    let all = rows(cx, &window);
    let at = all.iter().position(|line| line == "v docs").expect("docs is open");
    assert_eq!(all[at + 1], "(empty)");

    // A file made in the terminal shows after a refresh tick, while the tab is on show.
    fs::write(repo.join("docs/new.md"), b"").unwrap();
    // (The features' part of the window's tick: a whole tick would read the shell's real folder back.)
    drive(cx, &window, |ws, window, cx| ws.tick_features(window, cx));
    cx.run_until_parked();
    assert!(rows(cx, &window).contains(&"new.md".to_string()), "{:?}", rows(cx, &window));

    // Hidden, it reads nothing: a file made now is not there until the tab is shown again.
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Changes, cx));
    fs::write(repo.join("docs/later.md"), b"").unwrap();
    for _ in 0..3 {
        drive(cx, &window, |ws, window, cx| ws.tick_features(window, cx));
    }
    cx.run_until_parked();
    assert!(!rows(cx, &window).contains(&"later.md".to_string()));
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Files, cx));
    cx.run_until_parked();
    assert!(rows(cx, &window).contains(&"later.md".to_string()), "showing the tab looks again");

    // A folder that goes away while open takes its rows with it.
    fs::remove_dir_all(repo.join("docs")).unwrap();
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Files, cx));
    cx.run_until_parked();
    assert!(!rows(cx, &window).iter().any(|line| line.contains("docs")));
}

#[gpui_kit::test]
fn the_key_shows_the_tab_and_a_terminal_without_a_folder_says_so(cx: &mut TestAppContext) {
    if isolate("the_key_shows_the_tab_and_a_terminal_without_a_folder_says_so") {
        return;
    }
    let window = start(cx, 1);
    press(cx, &window, "ctrl-cmd-6");
    cx.run_until_parked();
    assert_eq!(read_cx(cx, &window, |ws, _| ws.right_tab()), TabId::Files, "⌃⌘6");
    assert!(drawn(cx, &window, "files-no-folder"), "the shell has not said where it is yet");
}

#[test]
fn a_file_is_opened_only_in_a_known_editor_and_a_folder_in_any_app() {
    let file = Row { path: "a.sh".into(), depth: 0, label: "a.sh".into(), kind: RowKind::File };
    let folder = Row { path: "d".into(), depth: 0, label: "d".into(), kind: RowKind::Folder { open: false } };
    assert!(open::may_open_in("dev.zed.Zed", &file) && open::may_open_in("com.apple.finder", &file));
    assert!(!open::may_open_in("com.apple.Terminal", &file), "a terminal would run it");
    assert!(open::may_open_in("com.apple.Terminal", &folder));
    assert_eq!(open::default_bundle("dev.zed.Zed"), "dev.zed.Zed");
    assert_eq!(open::default_bundle("com.googlecode.iterm2"), "com.apple.finder", "shown, not opened");
    assert_eq!(open::default_bundle(""), "com.apple.finder");
}

#[test]
fn a_path_with_a_control_character_or_that_is_not_text_is_not_pasteable() {
    assert_eq!(open::pasteable(Path::new("/w/my project/a.rs")), Some("/w/my project/a.rs"));
    for bad in ["/w/a\nrm -rf ~", "/w/a\tb", "/w/a\u{7f}", "/w/a\u{1b}[31m", "/w/a\u{85}"] {
        assert_eq!(open::pasteable(Path::new(bad)), None, "{bad:?}");
    }
    assert_eq!(open::pasteable(wormhole_core::platform::fs::path_from_bytes(b"/w/bad-\xff")), None);
}

#[gpui_kit::test]
fn finder_is_never_asked_to_open_a_folder_that_is_a_program(cx: &mut TestAppContext) {
    if isolate("finder_is_never_asked_to_open_a_folder_that_is_a_program") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    fs::create_dir(repo.join("x.app")).unwrap();
    go(cx, &window, &repo);
    open::record::take();
    for name in ["x.app", "docs"] {
        let target = row(cx, &window, name);
        cx.update(|cx| open::open_in("com.apple.finder", &repo, &target, cx));
        cx.run_until_parked();
    }
    let launched = open::record::take();
    assert_eq!(launched.len(), 2, "{launched:?}");
    for (bundle, launch) in &launched {
        assert!(
            matches!(launch, Launch::Reveal { .. }),
            "Finder ({bundle}) is shown a folder, never asked to open one: {launch:?}"
        );
    }
    // An editor opens a folder as a folder, `x.app` too.
    let target = row(cx, &window, "x.app");
    cx.update(|cx| open::open_in("dev.zed.Zed", &repo, &target, cx));
    cx.run_until_parked();
    assert!(matches!(open::record::take().as_slice(), [(_, Launch::Open { .. })]));
}

#[gpui_kit::test]
fn nothing_is_read_while_the_feature_is_off_even_with_the_tab_saved_as_shown(cx: &mut TestAppContext) {
    if isolate("nothing_is_read_while_the_feature_is_off_even_with_the_tab_saved_as_shown") {
        return;
    }
    let window = start(cx, 1);
    let repo = repo("project");
    go(cx, &window, &repo);
    fs::write(repo.join("new.md"), b"").unwrap();
    // The window as a release build draws it: the tab saved as the panel's, the feature off.
    let off = |cx: &mut TestAppContext, on: bool, again: bool| {
        drive(cx, &window, |ws, _, cx| {
            let mut facts = ws.facts(cx);
            facts.tab = TabId::Files;
            facts.features = facts.features.with(FeatureId::Files, on);
            feature(ws).update(cx, |files, fcx| {
                if again {
                    files.look_again(&facts, fcx);
                } else {
                    files.tick(&facts, fcx);
                }
            });
        });
    };
    off(cx, false, false);
    off(cx, false, true);
    assert!(!rows(cx, &window).contains(&"new.md".to_string()), "no read while the feature is off");
    off(cx, true, false);
    assert!(rows(cx, &window).contains(&"new.md".to_string()), "and the same tick reads when it is on");
}

#[gpui_kit::test]
fn an_answer_for_a_root_the_tree_has_left_is_dropped(cx: &mut TestAppContext) {
    use wormhole_core::storage::files::listing::{DirEntry, EntryKind, Listing};
    use wormhole_core::view::file_tree::{Dir, ENTRY_CAP};

    if isolate("an_answer_for_a_root_the_tree_has_left_is_dropped") {
        return;
    }
    let window = start(cx, 1);
    let (one, two) = (repo("one"), scratch("two"));
    go(cx, &window, &one);
    let old = read_cx(cx, &window, |ws, cx| {
        let session = ws.active_session().id;
        feature(ws).read(cx).of[&session].generation
    });
    go(cx, &window, &two);
    let ghost =
        Listing { entries: vec![DirEntry { name: "ghost".into(), kind: EntryKind::File }], truncated: false };
    drive(cx, &window, |ws, _, cx| {
        let session = ws.active_session().id;
        feature(ws).update(cx, |files, fcx| {
            files.in_flight.insert(one.clone());
            let answer = Some(Read { dir: Ok(Dir::from_listing(ghost, ENTRY_CAP)), stamp: None });
            files.arrived(session, old, &one, PathBuf::new(), answer, fcx);
            assert!(!files.in_flight.contains(&one), "the read is over, whoever wanted it");
        });
    });
    assert!(!rows(cx, &window).contains(&"ghost".to_string()), "the answer was for the old root");
}
