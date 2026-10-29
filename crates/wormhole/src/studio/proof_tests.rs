//! The Studio's save against real files in a scratch themes folder (`Themes::USER` with the
//! active profile pointed at it): what a person's existing theme files go through when they
//! are edited and saved. Each test builds its own folder under the temp dir.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gpui_kit::{AnyWindowHandle, AppContext as _, Context, Entity, TestAppContext, Window, WindowOptions};
use wormhole_core::config::tokens_from_json_in;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::theme::saved_versions::{list_versions, read_version};

use super::Edit;
use super::StudioWindow;
use super::document::Subject;
use super::opening::{Inputs, ProjectTheme, Target, open_document};
use super::themes::Themes;
use crate::settings::SettingsStore;

/// The active profile is process-wide, so these tests take turns.
static TURN: Mutex<()> = Mutex::new(());

struct Scratch {
    root: PathBuf,
    themes: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("wormhole-proof-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let profile = DataRoot::new(&root).default_profile();
        let themes = profile.themes_dir();
        std::fs::create_dir_all(&themes).expect("themes dir");
        crate::active_profile::set(profile);
        Self { root, themes }
    }

    fn put(&self, file: &str, text: &str) -> PathBuf {
        let path = self.themes.join(file);
        std::fs::write(&path, text).expect("write theme");
        path
    }

    fn text(&self, file: &str) -> String {
        std::fs::read_to_string(self.themes.join(file)).expect("read theme")
    }

    fn resolve(&self, file: &str) -> Result<wormhole_core::theme::tokens::ThemeTokens, String> {
        tokens_from_json_in(&self.themes, &self.text(file))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn open_on(
    cx: &mut TestAppContext,
    shown: &str,
    project: Option<ProjectTheme>,
) -> (AnyWindowHandle, Entity<StudioWindow>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        let in_use = cx.global::<SettingsStore>().base_tokens().clone();
        let doc = open_document(&Inputs { shown: shown.into(), project, in_use }, &Themes::USER);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| StudioWindow::with_themes(Themes::USER, doc, window, cx))
        })
        .expect("opens the window")
    })
}

fn with<R>(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    studio: &Entity<StudioWindow>,
    f: impl FnOnce(&mut StudioWindow, &mut Window, &mut Context<StudioWindow>) -> R,
) -> R {
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| studio.update(cx, |s, cx| f(s, window, cx)))
            .expect("the window is open")
    })
}

const DUSK: &str = r##"{
  // my theme
  "name": "Dusk",
  "extends": "Nord",
  "ui": { "primary": "#336699", "accent.hover": "#4477aa" },
  "terminal": { "background": "#101820" }
}
"##;

fn colors(t: &wormhole_core::theme::tokens::ThemeTokens, name: &str) -> String {
    t.get(name).map(|c| c.to_hex()).unwrap_or_default()
}

/// Saving a person's own older-format theme: the original is kept byte for byte, the new file
/// is version 1 and looks exactly like the old one with the edit applied.
#[gpui_kit::test]
fn saving_an_own_v0_theme_keeps_the_original_and_writes_v1(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("own");
    s.put("dusk.json", DUSK);
    // What the old release would show once the edit is applied, written the old way.
    let expect_text = DUSK.replace("#336699", "#aa0000");
    let expected = tokens_from_json_in(&s.themes, &expect_text).expect("v0 resolves");
    let (handle, studio) = open_on(cx, "Dusk", None);
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.doc.subject, Subject::Own { path: s.themes.join("dusk.json") });
        studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert!(!studio.dirty(), "{:?}", studio.notice);
    });
    let written = s.text("dusk.json");
    assert!(written.contains("\"version\": 1"), "{written}");
    let after = s.resolve("dusk.json").expect("v1 resolves");
    assert_eq!(super::tests_support_dump(&after), super::tests_support_dump(&expected));
    let id = "dusk";
    let versions = list_versions(&s.themes, id);
    assert_eq!(versions.len(), 1, "{versions:?}");
    assert!(versions[0].pinned, "the original is pinned");
    assert_eq!(read_version(&versions[0]).expect("reads"), DUSK, "byte for byte");
    assert_eq!(colors(&after, "primary"), "#aa0000");
}

/// A family file (a light and a dark version in one file) is never rewritten or flattened.
#[gpui_kit::test]
fn a_family_file_is_not_rewritten_by_a_studio_save(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("family");
    let pair = r##"{"name": "Pair", "extends": "Nord", "variants": [
        {"name": "Light", "appearance": "light", "terminal": {"background": "#f6f2ea", "foreground": "#2b2a35"}},
        {"name": "Dark", "appearance": "dark", "terminal": {"background": "#15131c", "foreground": "#ece6d8"}}]}"##;
    s.put("pair.json", pair);
    for shown in ["Pair Dark", "Pair Light", "pair"] {
        let (handle, studio) = open_on(cx, shown, None);
        with(cx, handle, &studio, |studio, window, cx| {
            let subject = studio.doc.subject.clone();
            studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
            studio.request_save(window, cx);
            // An untitled copy asks for a name and writes a NEW file; an Own subject would write over pair.json.
            if studio.naming.is_some() {
                studio.name.update(cx, |input, cx| input.set_value("Pair Copy", window, cx));
                studio.finish_rename(window, cx);
            }
            eprintln!("PROOF family pick {shown:?}: subject {subject:?}, notice {:?}", studio.notice);
        });
        assert_eq!(s.text("pair.json"), pair, "pair.json was rewritten after a save from the pick {shown:?}");
    }
}

/// A file another program changed since it was read is not overwritten without asking.
#[gpui_kit::test]
fn a_file_changed_on_disk_is_not_overwritten_without_asking(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("changed");
    s.put("dusk.json", DUSK);
    let (handle, studio) = open_on(cx, "Dusk", None);
    let theirs = DUSK.replace("#336699", "#00ff00");
    s.put("dusk.json", &theirs);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert!(studio.asking && studio.dirty());
    });
    assert!(cx.has_pending_prompt());
    assert_eq!(s.text("dusk.json"), theirs, "untouched while the question is up");
}

/// A write that fails leaves the original in place and the draft unsaved.
#[cfg(unix)]
#[gpui_kit::test]
fn a_failed_write_leaves_the_original_in_place(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("failed");
    s.put("dusk.json", DUSK);
    let (handle, studio) = open_on(cx, "Dusk", None);
    std::fs::set_permissions(&s.themes, std::fs::Permissions::from_mode(0o555)).expect("chmod");
    with(cx, handle, &studio, |studio, window, cx| {
        studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        eprintln!("PROOF failed write: notice {:?}, dirty {}", studio.notice, studio.dirty());
        assert!(studio.dirty(), "{:?}", studio.notice);
    });
    std::fs::set_permissions(&s.themes, std::fs::Permissions::from_mode(0o755)).expect("chmod back");
    assert_eq!(s.text("dusk.json"), DUSK, "original byte for byte");
    let leftovers: Vec<_> = std::fs::read_dir(&s.themes)
        .expect("dir")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert_eq!(leftovers, ["dusk.json"], "no stray temp files");
}

/// (a) A pick stored as an absolute file path opens that file as the person's own theme.
#[gpui_kit::test]
fn case_a_a_pick_stored_as_a_path_opens_the_file(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("pathpick");
    let path = s.put("single.json", DUSK);
    let (handle, studio) = open_on(cx, &path.to_string_lossy(), None);
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(
            studio.doc.subject,
            Subject::Own { path: path.clone() },
            "opened as {:?}",
            studio.doc.subject
        );
    });
}

/// (c) A v0 user theme "Nord" that extends "Nord": the Studio opens it, converts it, and the pick
/// "Nord" keeps meaning the same theme.
#[gpui_kit::test]
fn case_c_user_nord_extending_nord(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("nord");
    let text = r##"{"name": "Nord", "extends": "Nord", "ui": {"primary": "#ff0000"}}"##;
    s.put("nord.json", text);
    let before = s.resolve("nord.json").expect("v0");
    let (handle, studio) = open_on(cx, "Nord", None);
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.doc.subject, Subject::Own { path: s.themes.join("nord.json") });
        studio.set_value("primary", Some("#00ff00".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert!(!studio.dirty(), "{:?}", studio.notice);
    });
    let after = s.resolve("nord.json").expect("v1");
    assert_eq!(colors(&after, "primary"), "#00ff00");
    assert_eq!(colors(&before, "terminal.background"), colors(&after, "terminal.background"));
    assert_eq!(
        super::tests_support_dump(&before).replace("#ff0000", "#00ff00").len(),
        super::tests_support_dump(&after).len()
    );
    // The pick "Nord" still selects the user's file.
    let picked = wormhole_core::theme::registry::UserThemes::read(&s.themes).select("Nord");
    assert_eq!(picked, Some(wormhole_core::theme::registry::ThemeId::File(s.themes.join("nord.json"))));
}

/// (d) Hand-written ANSI keys by name are edited by index in the Studio; the saved file has one
/// value for that color, and every other named color survives.
#[gpui_kit::test]
fn case_d_named_ansi_keys_edited_in_the_studio(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("ansi");
    let text = r##"{"name": "Blue Named", "extends": "Nord", "terminal": {"ansi": {"blue": "#0000ff", "bright_blue": "#5555ff", "red": "#ff0000"}}}"##;
    s.put("blue.json", text);
    let (handle, studio) = open_on(cx, "Blue Named", None);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.set_value("terminal.ansi.4", Some("#123456".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert!(!studio.dirty(), "{:?}", studio.notice);
    });
    let after = s.resolve("blue.json").expect("resolves");
    assert_eq!(colors(&after, "terminal.ansi.4"), "#123456", "{}", s.text("blue.json"));
    assert_eq!(colors(&after, "terminal.ansi.12"), "#5555ff", "bright_blue kept");
    assert_eq!(colors(&after, "terminal.ansi.1"), "#ff0000", "red kept");
}

/// (e) A file with a newer version is never saved over.
#[gpui_kit::test]
fn case_e_a_version_2_file_is_not_saved_over(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("v2");
    let mut ansi = String::new();
    for i in 0..16 {
        ansi.push_str(&format!("\"{i}\": \"#{i:02x}{i:02x}{i:02x}\","));
    }
    let text = format!(
        r##"{{"version": 2, "id": "future", "name": "Future", "terminal": {{"foreground": "#ffffff", "background": "#000000", "cursor": "#ffffff", "selection_background": "#333333", "ansi": {{{}}}}}, "novel_section": {{"a": 1}}}}"##,
        ansi.trim_end_matches(',')
    );
    s.put("future.json", &text);
    let (handle, studio) = open_on(cx, "Future", None);
    with(cx, handle, &studio, |studio, window, cx| {
        eprintln!("PROOF v2: subject {:?}", studio.doc.subject);
        studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        eprintln!("PROOF v2: notice {:?} dirty {}", studio.notice, studio.dirty());
    });
    assert_eq!(s.text("future.json"), text, "a newer file was written over");
}

/// (f) Choosing the theme being edited in the theme menu while it has unsaved edits, then Save:
/// the edits end up in the file once, and nothing else is lost.
#[gpui_kit::test]
fn case_f_choosing_the_open_theme_with_edits_then_save(cx: &mut TestAppContext) {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let s = Scratch::new("reopen");
    let path = s.put("dusk.json", DUSK);
    let (handle, studio) = open_on(cx, "Dusk", None);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
        studio.open_target(&Target::Own(path.clone()), window, cx);
        eprintln!("PROOF f: asking {} dirty {}", studio.asking, studio.dirty());
    });
    if cx.has_pending_prompt() {
        let (message, _) = cx.pending_prompt().expect("the question");
        eprintln!("PROOF f: question {message:?}");
        cx.simulate_prompt_answer("Save");
        cx.run_until_parked();
    }
    with(cx, handle, &studio, |studio, window, cx| {
        eprintln!("PROOF f: after answer dirty {} subject {:?}", studio.dirty(), studio.doc.subject);
        studio.request_save(window, cx);
    });
    let after = s.resolve("dusk.json").expect("resolves");
    assert_eq!(colors(&after, "primary"), "#aa0000", "the edit reached the file: {}", s.text("dusk.json"));
    assert_eq!(list_versions(&s.themes, "dusk").len(), 1, "one kept version, not several");
}

/// A project theme (`.wormhole/theme.json`) stays in the older format: Studio writes it through
/// `export_json`, never a version 1 file, so older releases and other machines read it.
#[test]
fn a_project_theme_is_written_in_the_older_format() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _s = Scratch::new("project");
    let v0 = r##"{"name": "Project Theme", "extends": "Nord", "ui": {"primary": "#00aa88"}}"##;
    let file = wormhole_core::theme::tokens::parse_theme_file(v0).expect("parses");
    let resolve = Themes::USER.resolve;
    let out = super::draft::export_json(&file, resolve).expect("exports");
    for key in ["\"version\"", "\"base\"", "\"id\""] {
        assert!(!out.contains(key), "{key} in {out}");
    }
    // A converted (v1) theme exported for a project is standalone v0 too.
    let themes = Themes::USER;
    let v1 = (themes.convert)(&file, "project_theme").expect("converts");
    let out1 = super::draft::export_json(&v1, resolve).expect("exports");
    for key in ["\"version\"", "\"base\"", "\"id\"", "\"extends\""] {
        assert!(!out1.contains(key), "{key} in {out1}");
    }
    if let Ok(dir) = std::env::var("PROOF_OUT") {
        std::fs::write(Path::new(&dir).join("project_from_v0.json"), &out).expect("write");
        std::fs::write(Path::new(&dir).join("project_from_v1.json"), &out1).expect("write");
    }
}
