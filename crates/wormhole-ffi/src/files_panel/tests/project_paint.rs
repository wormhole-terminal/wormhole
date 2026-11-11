//! A folder's colour: the person's own, or the project's, which a lock and a bad file protect.

use std::fs;

use super::{Owner, Repo, service};
use crate::files_panel::{FolderPaintRecord, ProjectFailure};

fn paint(service: &crate::files_panel::ProjectService, folder: &str) -> FolderPaintRecord {
    service.folder_paint(folder.to_string())
}

#[test]
fn a_folder_with_no_project_wears_the_persons_own_colour() {
    // A folder of its own: the person's colours are in the experiment's `config.json`, shared by the process.
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join(".git")).expect("git");
    let folder = dir.path().canonicalize().expect("canonical").to_string_lossy().into_owned();
    let (_data, service) = service(&Owner::answering([]));
    let before = paint(&service, &folder);
    assert_eq!((before.chosen.clone(), before.custom.clone(), before.locked), (None, None, false));
    assert_eq!(before.note, "Marks this folder's band. Yours only: it isn't saved in the project.");
    service.set_folder_color(folder.clone(), Some("teal".into())).expect("colour");
    assert_eq!(paint(&service, &folder).chosen.as_deref(), Some("teal"));
    service.set_folder_color(folder.clone(), Some("#7fb2ff".into())).expect("hex");
    let hex = paint(&service, &folder);
    assert_eq!(
        (hex.chosen, hex.custom.as_deref()),
        (None, Some("#7fb2ff")),
        "a hex colour is none of the names"
    );
    assert!(matches!(
        service.set_folder_color(folder.clone(), Some("not a colour".into())),
        Err(ProjectFailure::Failed { .. })
    ));
    service.set_folder_color(folder.clone(), None).expect("taken away");
    assert_eq!(paint(&service, &folder).chosen, None);
    assert!(!dir.path().join(".wormhole").exists(), "nothing is written into the folder");
}

#[test]
fn a_projects_colour_is_changed_in_its_config_by_a_splice_that_keeps_the_comments() {
    let repo = Repo::new(&[("config.json", "// shared\n{\n  \"font_size\": 14, // keep\n}\n")]);
    let (_data, service) = service(&Owner::answering([]));
    let folder = repo.root();
    assert!(paint(&service, &folder).note.contains("Saved in the project's config.json"));
    service.set_folder_color(folder.clone(), Some("green".into())).expect("colour");
    let text = fs::read_to_string(repo.path().join(".wormhole/config.json")).expect("config");
    assert!(
        text.contains("// shared")
            && text.contains("// keep")
            && text.contains("\"project_color\": \"green\""),
        "{text}"
    );
    assert_eq!(paint(&service, &folder).chosen.as_deref(), Some("green"));
    service.set_folder_color(folder.clone(), None).expect("removed");
    assert!(
        !fs::read_to_string(repo.path().join(".wormhole/config.json"))
            .expect("config")
            .contains("project_color")
    );
}

#[test]
fn a_locked_or_unreadable_project_colour_is_refused_and_the_file_stays_as_it_was() {
    let locked = "{\"project_color\": \"red\", \"project_color_locked\": true}";
    let repo = Repo::new(&[("config.json", locked)]);
    let (_data, service) = service(&Owner::answering([]));
    let shown = paint(&service, &repo.root());
    assert!(shown.locked && shown.chosen.as_deref() == Some("red"));
    assert!(shown.note.contains("locks the colour"));
    assert!(matches!(
        service.set_folder_color(repo.root(), Some("blue".into())),
        Err(ProjectFailure::Refused { .. })
    ));
    assert_eq!(fs::read_to_string(repo.path().join(".wormhole/config.json")).expect("config"), locked);

    let broken = Repo::new(&[("config.json", "{ nope")]);
    let shown = paint(&service, &broken.root());
    assert!(shown.locked, "a config that can't be read can't be edited");
    assert!(matches!(
        service.set_folder_color(broken.root(), Some("blue".into())),
        Err(ProjectFailure::Refused { .. })
    ));
    assert_eq!(fs::read_to_string(broken.path().join(".wormhole/config.json")).expect("config"), "{ nope");
}

#[test]
fn a_config_that_links_out_of_the_project_is_not_written_through() {
    let outside = tempfile::tempdir().expect("temp dir");
    fs::write(outside.path().join("config.json"), "{}").expect("write");
    let repo = Repo::new(&[("notes.md", "")]);
    super::link(&outside.path().join("config.json"), &repo.path().join(".wormhole/config.json"));
    let (_data, service) = service(&Owner::answering([]));
    assert!(paint(&service, &repo.root()).locked, "it does not load, so nothing edits it");
    assert!(service.set_folder_color(repo.root(), Some("pink".into())).is_err());
    assert_eq!(fs::read_to_string(outside.path().join("config.json")).expect("read"), "{}");
}

#[test]
fn a_project_folder_is_made_once_and_never_over_something_that_is_there() {
    let dir = tempfile::tempdir().expect("temp dir");
    let folder = dir.path().to_string_lossy().into_owned();
    let (_data, service) = service(&Owner::answering([]));
    assert_eq!(service.create_project(folder.clone()), Ok(folder.clone()));
    let config = fs::read_to_string(dir.path().join(".wormhole/config.json")).expect("config");
    assert!(
        config.contains("project_color") && config.trim_start().starts_with("//"),
        "every key commented out"
    );
    assert!(dir.path().join(".wormhole/notes.md").exists());
    fs::write(dir.path().join(".wormhole/config.json"), "{\"font_size\": 20}").expect("edit");
    assert!(service.create_project(folder.clone()).is_ok(), "already a project: fine, and untouched");
    assert_eq!(
        fs::read_to_string(dir.path().join(".wormhole/config.json")).expect("config"),
        "{\"font_size\": 20}"
    );
    let missing = service.create_project("/nonexistent/Wormhole-swift-folder".into());
    assert!(matches!(missing, Err(ProjectFailure::Failed { .. })), "{missing:?}");
    let file = dir.path().join("a file");
    fs::write(&file, "x").expect("write");
    assert!(service.create_project(file.to_string_lossy().into_owned()).is_err());
}
