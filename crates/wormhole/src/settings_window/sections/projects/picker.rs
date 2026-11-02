//! The project select: "All projects" or one of the known folders, searchable.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::features::project;
use crate::select::{Choice, SearchSelect, SelectTrigger};
use crate::settings::UiColors;
use crate::settings_window::SettingsWindow;

/// The select's first choice: the defaults every project starts from.
pub(super) fn all_projects() -> &'static str {
    text(SETTINGS_PROJECTS_ALL_PROJECTS_DEFAULTS)
}

/// A project's name in the select: its folder's name.
pub(super) fn choice_label(root: &Path) -> SharedString {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into()
}

pub(super) fn choices(known: &[PathBuf]) -> Vec<Choice<String>> {
    let mut items = vec![Choice::new(String::new(), all_projects())];
    items.extend(known.iter().map(|root| {
        Choice::new(root.to_string_lossy().into_owned(), choice_label(root)).detail(short_home(root))
    }));
    items
}

pub(super) fn short_home(path: &Path) -> String {
    match crate::settings_window::home() {
        Some(home) if path.starts_with(&home) => {
            format!("~/{}", path.strip_prefix(&home).unwrap_or(path).display())
        }
        _ => path.display().to_string(),
    }
}

impl SettingsWindow {
    /// The project select: "All projects" or one of the known folders, searchable.
    pub(super) fn project_select(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let value = self.projects.selected.as_ref().map(|p| p.to_string_lossy().into_owned());
        let value = value.unwrap_or_default();
        // The trigger's own text; the options are only built while the menu is open.
        let label = match &self.projects.selected {
            Some(root) if self.projects.known.contains(root) => choice_label(root),
            _ => all_projects().into(),
        };
        let known = self.projects.known.clone();
        let this = cx.weak_entity();
        let on_change = this.clone();
        SearchSelect::new(
            "project-select",
            ui,
            SelectTrigger::new("project-select-trigger", ui, label).width(px(360.0)),
            &self.projects.search,
            cx,
        )
        .options_with(move || choices(&known).into())
        .current(value)
        .open(self.projects.open)
        .menu_width(px(360.0))
        .on_open_change(move |want_open, window, cx| {
            on_change
                .update(cx, |this, cx| {
                    this.projects.open = want_open;
                    if want_open {
                        this.projects.search.update(cx, |search, cx| search.set_value("", window, cx));
                    }
                    cx.notify();
                })
                .ok();
        })
        .on_pick(move |value, _, cx| {
            this.update(cx, |this, cx| {
                this.projects.pick(value);
                cx.notify();
            })
            .ok();
        })
        .into_popover()
        .into_any_element()
    }

    /// Re-list the projects the picker offers, keeping the selection.
    pub(in crate::settings_window) fn refresh_known_projects(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Listing checks the disk, so only while the list is on screen.
        if self.section != Section::Projects {
            return;
        }
        let known = project::known_projects(cx);
        if known != *self.projects.known {
            self.projects.known = Rc::new(known);
        }
        // The files and the trust status can change under us (an approval finishing, an
        // edit in the folder).
        self.projects.forget_info();
        cx.notify();
    }
}
