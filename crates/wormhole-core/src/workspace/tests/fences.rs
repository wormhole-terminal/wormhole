//! Groups that keep only the terminals in their folder: who they admit, who they let go, with a
//! disk made of a table.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::view::sidebar::Part;
use crate::workspace::{Action, DropPlan, Ground, ModelConfig, Over, WorkspaceModel};

use super::{listed, refused};

/// A disk where every path is a folder that is itself, and `/work/app` and `/work/app-wt` are two
/// worktrees of one repository.
#[derive(Debug)]
struct Table;

impl Ground for Table {
    fn real_folder(&self, path: &Path) -> Option<PathBuf> {
        Some(path.to_path_buf())
    }

    fn repo_of(&self, folder: &Path) -> Option<PathBuf> {
        folder.starts_with("/work/app").then(|| PathBuf::from("/work/app"))
    }

    fn worktree_of(&self, folder: &Path) -> Option<PathBuf> {
        folder.starts_with("/work/app-wt").then(|| PathBuf::from("/work/app"))
    }
}

fn model_at(places: &[&str]) -> WorkspaceModel {
    let mut m = WorkspaceModel::new(ModelConfig {
        numbers: crate::view::sidebar::SidebarNumbers::List,
        ground: Some(Arc::new(Table)),
        ..ModelConfig::default()
    });
    for place in places {
        m.apply(Action::NewTerminal);
        let id = *m.ids().last().expect("a terminal");
        m.update_facts(id, |f| f.cwd = Some(PathBuf::from(place)));
    }
    m
}

#[test]
fn turning_the_fence_on_takes_out_the_terminals_outside_the_folder() {
    let mut m = model_at(&["/work/app/src", "/home/me", "/work/app-wt/src"]);
    m.apply(Action::NewGroup { members: vec![1, 2, 3] });
    m.apply(Action::SetGroupFolder { group: 1, folder: Some("/work/app".into()) });
    m.apply(Action::SetOnlyInFolder { group: 1, on: true });
    // 2 is elsewhere and leaves; 3 is a worktree of the folder's repository and stays.
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "3:2", "2:3"]);
}

#[test]
fn a_fenced_group_admits_a_terminal_that_is_inside_and_refuses_one_that_is_not() {
    let mut m = model_at(&["/work/app/src", "/home/me", "/work/app/tests"]);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::SetGroupFolder { group: 1, folder: Some("/work/app".into()) });
    m.apply(Action::SetOnlyInFolder { group: 1, on: true });
    let onto = Over::Head { group: 1, part: Part::Middle };
    assert_eq!(m.drop_plan(2, onto), None, "outside: the drop has no landing there");
    assert!(m.drop_plan(3, onto).is_some(), "inside is taken");
    assert!(refused(&m.apply(Action::AddToGroup { id: 2, group: 1 })));
    m.apply(Action::AddToGroup { id: 3, group: 1 });
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "3:2", "2:3"]);
    // A terminal that walks into the folder is admitted, and one that walks out is let go.
    m.update_facts(2, |f| f.cwd = Some(PathBuf::from("/work/app/docs")));
    assert!(matches!(m.drop_plan(2, onto), Some(DropPlan::Move { .. })));
    m.update_facts(3, |f| f.cwd = Some(PathBuf::from("/tmp")));
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "3:2", "2:3"].map(String::from), "3 stands after the group");
}

#[test]
fn without_a_disk_no_group_fences_anything() {
    let mut m = super::model(2);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::SetGroupFolder { group: 1, folder: Some("/work/app".into()) });
    m.apply(Action::SetOnlyInFolder { group: 1, on: true });
    m.apply(Action::AddToGroup { id: 2, group: 1 });
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "2:2"]);
}

#[test]
fn a_group_with_no_folder_is_offered_the_one_its_terminals_share() {
    let mut m = WorkspaceModel::new(ModelConfig {
        home: Some("/Users/me".into()),
        ground: Some(Arc::new(Table)),
        ..ModelConfig::default()
    });
    for place in ["/Users/me/site/src", "/Users/me/site/tests"] {
        m.apply(Action::NewTerminal);
        let id = *m.ids().last().expect("a terminal");
        m.update_facts(id, |f| f.cwd = Some(PathBuf::from(place)));
    }
    m.apply(Action::NewGroup { members: vec![1, 2] });
    assert_eq!(m.group_folder_offer(1), Some(PathBuf::from("/Users/me/site")));
    m.apply(Action::DeclineGroupFolder(1));
    assert_eq!(m.group_folder_offer(1), None, "said no once: not asked again");
    m.apply(Action::SetGroupFolder { group: 1, folder: Some("/Users/me/site".into()) });
    assert_eq!(m.group_folder_offer(1), None, "it has one");
}
