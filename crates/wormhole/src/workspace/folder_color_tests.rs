//! View tests of a folder's colour and of the Project tab that sets it: whose colour a
//! folder's band wears, who has the tab, and making a project folder from it.

use std::path::{Path, PathBuf};

use gpui_kit::TestAppContext;
use wormhole_core::folder_color::FolderColor;
use wormhole_core::project::{
    Feature, ProjectColor, ProjectColorChoice, ProjectFolder, TrustStatus, TrustStore,
};

use super::app::OpenWindow;
use super::folder_color::{FolderPaint, ProjectPaint, set_folder_color};
use super::project::set_feature_for;
use super::view_tests::{draw, drive, isolate_in, read, start};
use super::{RightTab, Workspace};
use crate::settings::{SettingsStore, settings};
use crate::theme_bridge;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A folder inside the test's scratch dir (the working directory of the child process), with a
/// `.wormhole/config.json` that colours the project when `project_color` is given.
fn folder(name: &str, project_color: Option<&str>) -> PathBuf {
    let folder = std::env::current_dir().expect("the scratch dir").join(name);
    std::fs::create_dir_all(&folder).expect("mkdir");
    let folder = folder.canonicalize().expect("canonical");
    if let Some(color) = project_color {
        std::fs::create_dir_all(folder.join(".wormhole")).expect("mkdir");
        std::fs::write(folder.join(".wormhole/config.json"), format!(r#"{{ "project_color": "{color}" }}"#))
            .expect("write");
    }
    folder
}

/// The terminal in front is in `dir`, and the window has looked for its project.
fn go(ws: &mut Workspace, dir: &Path, cx: &mut gpui_kit::Context<Workspace>) {
    let dir = dir.to_path_buf();
    ws.active_terminal().update(cx, |terminal, _| terminal.cwd = Some(dir));
    ws.refresh_projects(cx);
}

/// The hex of the colour the active terminal's folder wears (its band's; the title row wears its
/// group's colour instead), if it has one.
fn tint(ws: &Workspace, _: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>) -> Option<String> {
    let tokens = &cx.global::<SettingsStore>().tokens;
    let paint = ws.folder_paint(&ws.active_folder(cx)?, cx);
    paint.tint(tokens).map(|(color, _)| color.color().to_hex())
}

/// The folder's paint, as the band and the Project tab get it.
fn paint(cx: &mut TestAppContext, window: &OpenWindow, folder: &Path) -> FolderPaint {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.folder_paint(folder, cx))).expect("open")
}

/// The words of the folder's tint for VoiceOver, if the folder is tinted.
fn band_name(cx: &mut TestAppContext, window: &OpenWindow, folder: &Path) -> Option<&'static str> {
    cx.update(|cx| {
        window.workspace.read_with(cx, |ws, cx| {
            let tokens = &cx.global::<SettingsStore>().tokens;
            ws.folder_paint(folder, cx).tint(tokens).map(|(_, name)| name)
        })
    })
    .expect("open")
}

/// Pick `color` for `folder` as its band's menu does (the colour goes where the paint says).
fn pick(cx: &mut TestAppContext, window: &OpenWindow, folder: &Path, color: Option<FolderColor>) {
    drive(cx, window, |ws, _, cx| {
        let paint = ws.folder_paint(folder, cx);
        set_folder_color(&paint, folder, color, cx);
    });
}

fn config_of(folder: &Path) -> String {
    std::fs::read_to_string(folder.join(".wormhole/config.json")).expect("config.json")
}

fn red_hex(cx: &mut TestAppContext) -> String {
    cx.update(|cx| {
        let tokens = &cx.global::<SettingsStore>().tokens;
        ProjectColor::from_color(FolderColor::Red.color(tokens)).color().to_hex()
    })
}

/// A project that has a `config.json` keeps the folder's colour there, and the person's own colour
/// for the folder is not looked at: choosing writes `project_color` and the title row follows it;
/// a folder in no project keeps the person's own colour as before.
#[gpui_kit::test]
fn a_projects_config_holds_its_folders_colour_and_the_persons_own_is_ignored(cx: &mut TestAppContext) {
    if isolate("a_projects_config_holds_its_folders_colour_and_the_persons_own_is_ignored") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let tinted = folder("tinted", Some("#2bb3a3"));
    let plain = folder("plain", None);

    let project = drive(cx, &window, |ws, window, cx| {
        go(ws, &tinted, cx);
        tint(ws, window, cx)
    });
    assert_eq!(project.as_deref(), Some("#2bb3a3"), "the project's own colour, as before");
    assert!(matches!(paint(cx, &window, &tinted), FolderPaint::Project(_)));

    // An old colour of the person's for this folder (from before it was a project) is ignored...
    cx.update(|cx| {
        super::settings_page::update_settings(cx, |settings| {
            settings.set_folder_color(&tinted, Some(FolderColor::Blue));
        });
    });
    let still = drive(cx, &window, |ws, window, cx| tint(ws, window, cx));
    assert_eq!(still.as_deref(), Some("#2bb3a3"), "one source of truth");
    assert_eq!(band_name(cx, &window, &tinted), Some("by its project"), "a hex colour has no name");

    // ...and a pick goes into the project's file, leaving the person's entry alone.
    pick(cx, &window, &tinted, Some(FolderColor::Red));
    assert_eq!(config_of(&tinted), "{ \"project_color\": \"red\" }");
    let red = red_hex(cx);
    let own = drive(cx, &window, |ws, window, cx| tint(ws, window, cx));
    assert_eq!(own.as_deref(), Some(red.as_str()), "the title row follows the file at once");
    assert_eq!(band_name(cx, &window, &tinted), Some("Red"));
    let kept = cx.update(|cx| settings(cx).folder_color(&tinted));
    assert_eq!(kept, Some(FolderColor::Blue), "the person's entry is kept for if the project goes away");

    // None takes the key out; the person's blue stays ignored.
    pick(cx, &window, &tinted, None);
    assert_eq!(config_of(&tinted), "{ }");
    let none = drive(cx, &window, |ws, window, cx| tint(ws, window, cx));
    assert_eq!(none, None);
    assert_eq!(band_name(cx, &window, &tinted), None);

    // A folder in no project: the person's own colour wins and is saved in their settings.
    let other = drive(cx, &window, |ws, window, cx| {
        go(ws, &plain, cx);
        tint(ws, window, cx)
    });
    assert_eq!(other, None);
    assert!(matches!(paint(cx, &window, &plain), FolderPaint::Own(None)));
    pick(cx, &window, &plain, Some(FolderColor::Red));
    let own = drive(cx, &window, |ws, window, cx| tint(ws, window, cx));
    assert_eq!(own.as_deref(), Some(red.as_str()));
    assert_eq!(band_name(cx, &window, &plain), Some("Red"));
    assert!(!plain.join(".wormhole").exists(), "nothing was written into the folder");
    pick(cx, &window, &plain, None);
    let colours = cx.update(|cx| settings(cx).folder_colors.len());
    assert_eq!(colours, 1, "only the ignored entry for the project is left");
}

/// Switching the project-folder flag off takes effect at once, before the window's next look for
/// projects: the folder wears only the person's colour, the Project tab is gone, and a colour picked
/// in a menu opened while the flag was on goes into the person's settings, never the repository.
#[gpui_kit::test]
fn with_the_project_folder_flag_off_nothing_is_written_into_the_repository(cx: &mut TestAppContext) {
    if isolate("with_the_project_folder_flag_off_nothing_is_written_into_the_repository") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let tinted = folder("tinted", Some("teal"));
    drive(cx, &window, |ws, _, cx| go(ws, &tinted, cx));
    // The band's menu was opened while the feature was on: it holds the project's paint.
    let opened = paint(cx, &window, &tinted);
    assert!(matches!(opened, FolderPaint::Project(_)));

    // Off, with no `refresh_projects` since: the window still lists the project.
    cx.update(|cx| {
        super::settings_page::update_settings(cx, |settings| {
            settings.feature_flags.insert("project_folders".into(), false);
        });
    });
    assert!(matches!(paint(cx, &window, &tinted), FolderPaint::Own(None)), "the person's colour only");
    let tinted_row = drive(cx, &window, |ws, window, cx| tint(ws, window, cx));
    assert_eq!(tinted_row, None, "the project's teal is gone from the title row");
    let tabs = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.panel_tab_list(cx))).expect("open");
    assert!(!tabs.contains(&RightTab::Project));

    drive(cx, &window, |_, _, cx| set_folder_color(&opened, &tinted, Some(FolderColor::Red), cx));
    assert_eq!(config_of(&tinted), r#"{ "project_color": "teal" }"#, "the repository is untouched");
    let own = cx.update(|cx| settings(cx).folder_color(&tinted));
    assert_eq!(own, Some(FolderColor::Red), "the pick is the person's own colour");
}

/// A project folder without a `config.json` has no place for a colour: the person's own is used.
#[gpui_kit::test]
fn a_project_folder_without_a_config_json_keeps_the_persons_own_colour(cx: &mut TestAppContext) {
    if isolate("a_project_folder_without_a_config_json_keeps_the_persons_own_colour") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let bare = folder("bare-project", None);
    std::fs::create_dir_all(bare.join(".wormhole")).expect("mkdir");
    std::fs::write(bare.join(".wormhole/notes.md"), "").expect("write");
    drive(cx, &window, |ws, _, cx| go(ws, &bare, cx));
    assert!(matches!(paint(cx, &window, &bare), FolderPaint::Own(None)));
    pick(cx, &window, &bare, Some(FolderColor::Teal));
    assert!(matches!(
        paint(cx, &window, &bare),
        FolderPaint::Own(Some(ProjectColorChoice::Named(FolderColor::Teal)))
    ));
    assert!(!bare.join(".wormhole/config.json").exists(), "no config.json is made for a colour");

    // The picker's colour is a hex colour, kept the same way and drawn as it is.
    let picked = ProjectColor::parse("#2bb3a3").expect("a colour");
    drive(cx, &window, |ws, _, cx| {
        ws.pick_folder_color(theme_bridge::hsla(picked.color()), cx);
    });
    let now = paint(cx, &window, &bare);
    assert!(matches!(now, FolderPaint::Own(Some(ProjectColorChoice::Hex(c))) if c == picked), "{now:?}");
    assert_eq!(now.custom(), Some(picked));
    assert!(!bare.join(".wormhole/config.json").exists(), "no config.json is made for a colour");
}

/// `project_color_locked: true`: the menus' choices are closed with the project's words, a pick
/// changes nothing, and the colour still shows.
#[gpui_kit::test]
fn a_locked_colour_is_shown_and_cannot_be_picked(cx: &mut TestAppContext) {
    if isolate("a_locked_colour_is_shown_and_cannot_be_picked") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let locked = folder("locked", None);
    let text = "{ \"project_color\": \"teal\", \"project_color_locked\": true }";
    std::fs::create_dir_all(locked.join(".wormhole")).expect("mkdir");
    std::fs::write(locked.join(".wormhole/config.json"), text).expect("write");
    let teal = cx.update(|cx| {
        ProjectColor::from_color(FolderColor::Teal.color(&cx.global::<SettingsStore>().tokens))
            .color()
            .to_hex()
    });
    let shown = drive(cx, &window, |ws, window, cx| {
        go(ws, &locked, cx);
        tint(ws, window, cx)
    });
    assert_eq!(shown.as_deref(), Some(teal.as_str()));
    let paint_now = paint(cx, &window, &locked);
    assert_eq!(paint_now.closed(), Some("Set by the project"));
    assert_eq!(paint_now.checked(), Some(FolderColor::Teal));
    for color in [Some(FolderColor::Red), None] {
        pick(cx, &window, &locked, color);
        assert_eq!(config_of(&locked), text, "{color:?}: the file is as it was");
    }
    // Locking it by hand while a menu is open: the pick it was offered is refused by the file.
    let open = paint(cx, &window, &locked);
    std::fs::write(locked.join(".wormhole/config.json"), "{ \"project_color\": \"teal\" }").expect("write");
    drive(cx, &window, |ws, _, cx| ws.refresh_projects(cx));
    assert_eq!(paint(cx, &window, &locked).closed(), None, "unlocked by hand");
    std::fs::write(locked.join(".wormhole/config.json"), text).expect("write");
    cx.update(|cx| {
        set_folder_color(
            &FolderPaint::Project(match &open {
                FolderPaint::Project(p) => ProjectPaint { locked: false, ..p.clone() },
                _ => unreachable!("a project"),
            }),
            &locked,
            Some(FolderColor::Red),
            cx,
        )
    });
    cx.run_until_parked();
    assert_eq!(config_of(&locked), text, "the file's lock holds whatever the menu thought");
    let error = cx.update(|cx| super::app(cx).hub.error.clone());
    assert!(error.is_some_and(|e| e.contains("locks its colour")), "and says so in the Project tab");
}

/// A `config.json` that doesn't load can't be edited: the choices are closed and nothing is
/// written.
#[gpui_kit::test]
fn a_config_that_does_not_load_is_not_rewritten(cx: &mut TestAppContext) {
    if isolate("a_config_that_does_not_load_is_not_rewritten") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let broken = folder("broken", None);
    std::fs::create_dir_all(broken.join(".wormhole")).expect("mkdir");
    std::fs::write(broken.join(".wormhole/config.json"), "{ \"font_size\": ").expect("write");
    drive(cx, &window, |ws, _, cx| go(ws, &broken, cx));
    assert_eq!(paint(cx, &window, &broken).closed(), Some("config.json can't be read"));
    pick(cx, &window, &broken, Some(FolderColor::Red));
    assert_eq!(config_of(&broken), "{ \"font_size\": ");
    // Even with the file fixed under a stale menu it is the file that is checked, not the menu.
    std::fs::write(broken.join(".wormhole/config.json"), "[1]").expect("write");
    let stale = paint(cx, &window, &broken);
    cx.update(|cx| {
        set_folder_color(
            &FolderPaint::Project(ProjectPaint {
                root: broken.clone(),
                chosen: None,
                locked: false,
                shown: true,
                unreadable: false,
            }),
            &broken,
            Some(FolderColor::Red),
            cx,
        )
    });
    cx.run_until_parked();
    drop(stale);
    assert_eq!(config_of(&broken), "[1]", "not an object: refused");
    let error = cx.update(|cx| super::app(cx).hub.error.clone());
    assert!(error.is_some_and(|e| e.starts_with("Couldn't change the project's colour")));
}

/// A project colour follows the project's Settings switch (it needs no trust); the band and the
/// title row show nothing while it is off, and picking still writes the file.
#[gpui_kit::test]
fn a_project_colour_follows_the_settings_switch_and_needs_no_trust(cx: &mut TestAppContext) {
    if isolate("a_project_colour_follows_the_settings_switch_and_needs_no_trust") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let project = folder("switch", Some("teal"));
    let shown = drive(cx, &window, |ws, window, cx| {
        go(ws, &project, cx);
        tint(ws, window, cx)
    });
    assert!(shown.is_some(), "untrusted, and tinted all the same");
    assert_eq!(band_name(cx, &window, &project), Some("Teal"));

    cx.update(|cx| set_feature_for(&project, Feature::Settings, Some(false), cx));
    cx.run_until_parked();
    let off = drive(cx, &window, |ws, window, cx| {
        ws.refresh_projects(cx);
        tint(ws, window, cx)
    });
    assert_eq!(off, None);
    assert_eq!(band_name(cx, &window, &project), None);
    let hidden = paint(cx, &window, &project);
    assert!(hidden.note().contains("Not shown while Settings is switched off"), "{}", hidden.note());
    assert_eq!(hidden.closed(), None, "it can still be picked");
    pick(cx, &window, &project, Some(FolderColor::Pink));
    assert_eq!(config_of(&project), "{ \"project_color\": \"pink\" }");

    cx.update(|cx| set_feature_for(&project, Feature::Settings, None, cx));
    cx.run_until_parked();
    let on = drive(cx, &window, |ws, window, cx| {
        ws.refresh_projects(cx);
        tint(ws, window, cx)
    });
    assert!(on.is_some());
    assert_eq!(band_name(cx, &window, &project), Some("Pink"));
}

/// A trusted project stays trusted when its colour is picked (the colour is not part of what is
/// approved), and a project that was never trusted does not become so.
#[gpui_kit::test]
fn picking_a_colour_does_not_send_a_trusted_project_back_for_approval(cx: &mut TestAppContext) {
    if isolate("picking_a_colour_does_not_send_a_trusted_project_back_for_approval") {
        return;
    }
    let window = start(cx, 2);
    draw(cx, &window);
    let trusted = folder("trusted", None);
    std::fs::create_dir_all(trusted.join(".wormhole")).expect("mkdir");
    std::fs::write(
        trusted.join(".wormhole/config.json"),
        "// our project\n{\n  \"env\": { \"RUST_LOG\": \"debug\" }\n}\n",
    )
    .expect("write");
    let status = |cx: &mut TestAppContext| {
        cx.update(|cx| super::app(cx).hub.projects.get(&trusted).map(|project| project.status))
    };
    drive(cx, &window, |ws, _, cx| go(ws, &trusted, cx));
    assert_eq!(status(cx), Some(TrustStatus::Untrusted));
    pick(cx, &window, &trusted, Some(FolderColor::Green));
    assert_eq!(status(cx), Some(TrustStatus::Untrusted), "a colour approves nothing");

    // Approved as the Touch ID answer would (no prompt in a test).
    cx.update(|cx| {
        let project = ProjectFolder::open(&trusted).expect("a project");
        super::app_mut(cx).hub.trust.trust(&project);
    });
    drive(cx, &window, |ws, _, cx| ws.refresh_projects(cx));
    assert_eq!(status(cx), Some(TrustStatus::Trusted));
    for color in [Some(FolderColor::Blue), None, Some(FolderColor::Pink)] {
        pick(cx, &window, &trusted, color);
        assert_eq!(status(cx), Some(TrustStatus::Trusted), "{color:?}");
    }
    assert!(config_of(&trusted).starts_with("// our project\n{\n  \"env\": { \"RUST_LOG\": \"debug\" },"));
    // Anything else in the file still asks.
    std::fs::write(trusted.join(".wormhole/config.json"), config_of(&trusted).replace("debug", "trace"))
        .expect("write");
    drive(cx, &window, |ws, _, cx| ws.refresh_projects(cx));
    assert_eq!(status(cx), Some(TrustStatus::Changed));
}

/// Any terminal with a folder has the Project tab, project folder or not, and creating one from
/// the tab makes `.wormhole/config.json` and turns the tab into the project's.
#[gpui_kit::test]
fn the_project_tab_is_there_for_any_folder_and_makes_the_project(cx: &mut TestAppContext) {
    if isolate("the_project_tab_is_there_for_any_folder_and_makes_the_project") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let tabs = |cx: &mut TestAppContext| {
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.panel_tab_list(cx))).expect("open")
    };
    assert!(!tabs(cx).contains(&RightTab::Project), "no folder yet: the shell hasn't said where it is");

    let bare = folder("bare", None);
    drive(cx, &window, |ws, _, cx| go(ws, &bare, cx));
    assert!(tabs(cx).contains(&RightTab::Project), "a folder is enough, with no .wormhole/");
    assert!(read(cx, &window, |ws| ws.active_session().project.clone().is_none()));

    drive(cx, &window, |ws, _, cx| ws.create_project_folder(cx));
    assert!(bare.join(".wormhole/config.json").is_file(), "the project folder was made");
    let found = read(cx, &window, |ws| ws.active_session().project.clone());
    assert_eq!(found.as_deref(), Some(bare.as_path()), "every window looked again at once");

    // A second click (or another program) finds it there: no error, nothing replaced.
    let config = std::fs::read_to_string(bare.join(".wormhole/config.json")).expect("config");
    drive(cx, &window, |ws, _, cx| ws.create_project_folder(cx));
    assert_eq!(std::fs::read_to_string(bare.join(".wormhole/config.json")).expect("config"), config);
    let error = cx.update(|cx| super::app(cx).hub.error.is_some());
    assert!(!error, "it is a project all the same");
}

/// An approval stored by the build before the colour left the trust fingerprint still holds when
/// the project is loaded, is rewritten on disk in the current form (keeping when it was approved),
/// and so survives a colour being picked and the app being started again.
#[gpui_kit::test]
fn an_older_approval_is_upgraded_on_load_and_survives_a_colour(cx: &mut TestAppContext) {
    if isolate("an_older_approval_is_upgraded_on_load_and_survives_a_colour") {
        return;
    }
    /// What the previous build fingerprinted this project as (taken from it, not from this code).
    const OLD: &str = "28e2e65fd85b6a2c9582f8adf71c9a9949536bf9e58316dee803340501eb665d";
    let window = start(cx, 1);
    draw(cx, &window);
    let project = folder("older", None);
    std::fs::create_dir_all(project.join(".wormhole")).expect("mkdir");
    std::fs::write(
        project.join(".wormhole/config.json"),
        "// Wormhole project\n{\n  \"font_size\": 13,\n  \"env\": { \"A\": \"1\" },\n  \"shell\": \"/bin/zsh\",\n  \"project_color\": \"#2bb3a3\"\n}\n",
    )
    .expect("write");
    let data = wormhole_core::profile::data_dir();
    let file = data.join(TrustStore::FILE_NAME);
    let key = project.to_string_lossy().into_owned();
    std::fs::write(
        &file,
        format!(r#"{{ {key:?}: {{ "fingerprint": "{OLD}", "trusted_at": 1700000000 }} }}"#),
    )
    .expect("write");
    cx.update(|cx| super::app_mut(cx).hub.trust = TrustStore::load(&data).expect("loads"));

    let status =
        |cx: &mut TestAppContext| cx.update(|cx| super::app(cx).hub.projects.get(&project).map(|p| p.status));
    drive(cx, &window, |ws, _, cx| go(ws, &project, cx));
    assert_eq!(status(cx), Some(TrustStatus::Trusted), "not asked to approve it again");
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("saved")).expect("json");
    let entry = &saved[key.as_str()];
    assert_ne!(entry["fingerprint"], OLD, "rewritten in the current form");
    assert_eq!(entry["trusted_at"], 1_700_000_000, "approved when it was approved");

    pick(cx, &window, &project, Some(FolderColor::Blue));
    assert_eq!(status(cx), Some(TrustStatus::Trusted));
    // As after a restart: the file alone says it is trusted.
    let restarted = TrustStore::load(&data).expect("loads");
    assert_eq!(restarted.status(&ProjectFolder::open(&project).expect("a project")), TrustStatus::Trusted);
    // Something else in the file is still a change.
    std::fs::write(
        project.join(".wormhole/config.json"),
        std::fs::read_to_string(project.join(".wormhole/config.json"))
            .expect("read")
            .replace("/bin/zsh", "/bin/sh"),
    )
    .expect("write");
    drive(cx, &window, |ws, _, cx| ws.refresh_projects(cx));
    assert_eq!(status(cx), Some(TrustStatus::Changed));
}
