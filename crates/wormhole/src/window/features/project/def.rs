//! The project folder's entry in the feature table: the Project tab, offered for any terminal that has a
//! folder (a project or not: it is where a folder gets its colour and, without one, where the project
//! folder is made), the key and menu lines for the project's actions, the look for each terminal's
//! project on the tick and when the window comes forward, what a new terminal starts with, and each
//! profile's hub. Each hook takes the window's entity once and downcasts it.
use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::project::Feature;
use wormhole_core::view::spoken::{self, PanelTab};

use super::{Project, detect, hub, hubs, panel, project_folders_on};
use crate::actions::RunProjectAction;
use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{FeatureDef, ProfileHooks, Root, StateDef, TabDef};

/// The window's `Project`, from the handle the window keeps. The entry makes it, so it is one.
fn typed(entity: &AnyEntity) -> Entity<Project> {
    entity.clone().downcast::<Project>().expect("the project entity is made by this entry's `make`")
}

pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Project,
    tabs: &[TabDef {
        id: TabId::Project,
        icon: IconName::Folder,
        label: "Project",
        action: ActionId::ShowProject,
        on_show: Some(|entity, facts, _, cx| look(entity, facts, cx)),
        // Whenever the terminal has a folder: its colour is set there, and its project made. It stays while
        // it is the tab on show, so a terminal whose folder isn't known for a moment doesn't pull it out
        // from under the person.
        shown: |facts, cx| {
            facts.tab == TabId::Project || facts.active().is_some_and(|session| session.folder(cx).is_some())
        },
        count: None,
        spoken: |_, _, _| spoken::panel_tab(PanelTab::Project),
        view: panel::tab,
        walk: None,
        actions: None,
        refresh_ticks: None,
        returns_to_previous: false,
        window_side: None,
    }],
    bind: Some(bind),
    row_extras: None,
    close_stake: None,
    menu: Some(menu),
    tick: Some(tick),
    profile_shown: Some(hubs::profile_shown),
    profiles: Some(ProfileHooks {
        opened: Some(hubs::profile_opened),
        deleted: Some(hubs::profile_deleted),
        id_allocated: Some(hubs::id_allocated),
    }),
    refresh: Some(look),
    trusted: Some(trusted),
    spawn: Some(detect::spawn_options),
    state: Some(STATE),
};

/// The window looks for each terminal's project again.
fn look(entity: &AnyEntity, facts: &WindowFacts<'_>, cx: &mut App) {
    typed(entity).update(cx, |project, cx| project.refresh(facts, cx));
}

/// The key of a project's action: it runs it in the terminal on screen at this frame, when the project
/// is trusted and has Actions on.
fn bind(entity: &AnyEntity, facts: &WindowFacts<'_>, root: Root, _: &App) -> Root {
    let Some(active) = facts.active() else { return root };
    let (project, session) = (typed(entity), active.id);
    root.on_action(move |action: &RunProjectAction, _, cx| {
        project.update(cx, |project, cx| project.run_action(session, &action.0, cx));
    })
}

/// The Project tab's menu line needs a folder and the feature; the menus list the actions of a trusted
/// project that has Actions on.
fn menu(
    entity: &AnyEntity,
    facts: &WindowFacts<'_>,
    state: &mut wormhole_core::keys::keymap::menus::MenuState,
    cx: &App,
) {
    let active = facts.active();
    if active.is_none_or(|session| session.folder(cx).is_none()) || !project_folders_on(cx) {
        state.disabled.push(ActionId::ShowProject);
    }
    let Some(active) = active else { return };
    let project = typed(entity);
    if let Some(project) =
        project.read(cx).project(active.id, cx).filter(|project| hub(cx).enabled(project, Feature::Actions))
    {
        state.project_actions = project.actions().iter().map(|a| (a.id.clone(), a.title.clone())).collect();
    }
}

/// Once per tick: the look for projects every fifth, the notes.md sync every third, and the project in
/// front counts as recently used.
fn tick(entity: &AnyEntity, facts: &WindowFacts<'_>, window: &mut Window, cx: &mut App) {
    typed(entity).update(cx, |project, cx| project.tick(facts, window, cx));
}

/// Whether terminal `session` is in a project the person has approved: what a program started for the
/// project's sake (git's pull request lookup starts `gh`) asks first.
fn trusted(entity: &AnyEntity, session: i64, cx: &App) -> bool {
    typed(entity).read(cx).project(session, cx).is_some_and(|project| hub(cx).trusted(project))
}

const STATE: StateDef = StateDef {
    make: |window, cx| {
        let entity = cx.new(|cx| Project::new(window, cx));
        Workspace::listen(&entity, window, cx)
    },
    selected: None,
    focus_in: None,
    // A terminal's project is looked for, never saved: nothing of the record is the project's.
    record: |_, _, _, _| {},
    opened: |entity, record, _, cx| typed(entity).update(cx, |project, _| project.open(record.id)),
    closed: |entity, session, cx| typed(entity).update(cx, |project, _| project.forget(session)),
    // Nothing is stored beside the record, so leaving and closing are the same here.
    left: |entity, session, cx| typed(entity).update(cx, |project, _| project.forget(session)),
};
