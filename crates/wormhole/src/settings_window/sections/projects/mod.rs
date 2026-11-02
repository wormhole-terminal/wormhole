//! Project folders: a project picker, then either the defaults for all
//! projects (plus the trusted list) or one project's own switches and trust.
//!
//! `picker` is the select, `defaults` the all-projects view, `one` one project's, `features` the
//! rows they share.

mod defaults;
mod features;
mod one;
mod picker;

use std::cell::{OnceCell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

use self::picker::choices;
use crate::features::project;
use crate::features::project::LoadedProject;
use crate::select;
use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges};

pub(super) const SETTINGS: &[Setting] = &[Setting::block("projects", |w, cx| w.draw_projects(cx))];

/// The picker's state inside the settings window.
pub(crate) struct ProjectPicker {
    /// `None` = the defaults for all projects.
    selected: Option<PathBuf>,
    known: Rc<Vec<PathBuf>>,
    open: bool,
    search: Entity<InputState>,
    /// The selected project's files as read for this page. Reading them (and the trust
    /// fingerprint) is file I/O, so it is kept between frames and read again about once a
    /// second, and right after a button changes something.
    info: RefCell<Option<(PathBuf, Option<Rc<LoadedProject>>)>>,
    /// Whether the Mac can approve a project (asked once; asking creates a system
    /// authentication context).
    can_approve: OnceCell<bool>,
}

impl ProjectPicker {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> (Self, Vec<Subscription>) {
        let known = Rc::new(project::known_projects(cx));
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(text(SETTINGS_PROJECTS_SEARCH_PROJECTS)));
        let subscriptions =
            vec![cx.subscribe_in(&search, window, |this, _, event: &InputEvent, _, cx| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter { .. } => {
                    this.projects.pick_first_match(cx);
                    cx.notify();
                }
                _ => {}
            })];
        (
            Self {
                selected: None,
                known,
                open: false,
                search,
                info: RefCell::new(None),
                can_approve: OnceCell::new(),
            },
            subscriptions,
        )
    }

    /// Jump to a project (e.g. from the trusted list).
    pub(crate) fn select(&mut self, root: PathBuf) {
        if !self.known.contains(&root) {
            Rc::make_mut(&mut self.known).push(root.clone());
        }
        self.selected = Some(root);
    }

    /// devctl: open the menu, optionally with `query` typed into its search box.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_open(&mut self, query: String, window: &mut Window, cx: &mut App) {
        self.open = true;
        self.search.update(cx, |search, cx| search.set_value(query, window, cx));
    }

    /// The project's files as last read, reading them if they are not (or are of another
    /// project).
    fn info(&self, root: &Path, cx: &App) -> Option<Rc<LoadedProject>> {
        let mut info = self.info.borrow_mut();
        if let Some((cached, project)) = info.as_ref()
            && cached == root
        {
            return project.clone();
        }
        let project = project::load_project_info(project::shown_profile(cx), root, cx).map(Rc::new);
        *info = Some((root.to_path_buf(), project.clone()));
        project
    }

    /// Something changed in the project (trust, a feature): read it again at the next draw.
    pub(crate) fn forget_info(&self) {
        self.info.take();
    }

    /// The choice's value is the project root ("" = all projects).
    fn pick(&mut self, value: &str) {
        self.selected = Some(value).filter(|v| !v.is_empty()).map(PathBuf::from);
        self.open = false;
    }

    /// Enter in the search box: take the first project that matches.
    fn pick_first_match(&mut self, cx: &App) {
        let query = self.search.read(cx).value().to_string();
        if let Some(first) = select::filter(&choices(&self.known), &query).into_iter().next() {
            self.pick(&first.value);
        }
    }
}

impl SettingsWindow {
    pub(super) fn draw_projects(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let all_projects = self.projects.selected.is_some().then(|| {
            style::link_button("all-projects", ui, text(SETTINGS_PROJECTS_ALL_PROJECTS), false)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.projects.selected = None;
                    cx.notify();
                }))
                .into_any_element()
        });
        let picker = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_PROJECT), None))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .py_2()
                    .rule_top(ui)
                    .child(self.project_select(ui, cx))
                    .children(all_projects),
            );
        let body = match self.projects.selected.clone() {
            None => self.render_project_defaults(cx),
            Some(root) => self.render_project(&root, cx),
        };
        div().flex().flex_col().gap_6().child(picker).child(body).into_any_element()
    }
}
