//! Using the theme being edited: whether it is the one the app shows, making it the pick of its
//! appearance (saving first when there is something to save), and, for a pair, following the
//! system's light and dark with both picks.

use gpui_kit::*;
use wormhole_core::config::ThemeMode;
use wormhole_core::theme::tokens::Appearance;
use wormhole_core::theme::tokens::appearance::{DARK, LIGHT};
use wormhole_core::theme::tokens::slug::version_slug;
use wormhole_text::keys::studio_pair_used;

use super::StudioWindow;
use super::document::Subject;
use super::save::Notice;
use crate::settings::{SettingsStore, picks, settings};

impl StudioWindow {
    /// What a pick holds to select the theme being edited as the appearance `shown` of it: the id
    /// of a pair's appearance, else the file.
    pub(super) fn pick_value(&self, shown: Option<&str>) -> Option<String> {
        let Subject::Own { path } = &self.doc.subject else { return None };
        match (&self.doc.file.id, shown, self.doc.is_pair()) {
            (Some(id), Some(shown), true) => Some(version_slug(id, shown)),
            _ => Some(path.to_string_lossy().into_owned()),
        }
    }

    /// Whether the app shows this theme (apart from the edits being previewed). Read from what was
    /// worked out when the document or the pick last changed: finding out whether a pick names a
    /// file reads the themes folder, which a draw never does.
    pub(super) fn in_use(&self, cx: &App) -> bool {
        match &self.doc.subject {
            Subject::Own { .. } => self.own_in_use,
            Subject::Untitled { from } => {
                let store = cx.global::<SettingsStore>();
                !self.dirty()
                    && settings(cx).shown_theme(store.system().appearance).eq_ignore_ascii_case(from)
            }
            // The project's theme is in effect while it is edited.
            Subject::Project { .. } => true,
        }
    }

    /// The pick the app shows now.
    fn current_pick(cx: &App) -> String {
        let store = cx.global::<SettingsStore>();
        settings(cx).shown_theme(store.system().appearance).to_string()
    }

    /// Work out again whether a pick names the file being edited (see [`Self::in_use`]).
    pub(super) fn refresh_own_in_use(&mut self, cx: &App) {
        let pick = Self::current_pick(cx);
        self.own_in_use = match &self.doc.subject {
            Subject::Own { path } => {
                let selects = self.themes.selects;
                selects(&pick, path)
                    || self.doc.file.id.as_deref().is_some_and(|id| {
                        pick == id || pick.strip_prefix(id).is_some_and(|rest| rest.starts_with('.'))
                    })
            }
            _ => false,
        };
        self.shown_pick = pick;
    }

    /// The settings changed: only another pick is worth a look (an edit's preview changes them
    /// too, many times a second).
    pub(super) fn settings_changed(&mut self, cx: &mut Context<Self>) {
        if Self::current_pick(cx) != self.shown_pick {
            self.refresh_own_in_use(cx);
            cx.notify();
        }
    }

    /// Use: the theme becomes the pick of its appearance; one with edits is saved first.
    pub(super) fn use_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dirty() || matches!(self.doc.subject, Subject::Untitled { .. }) {
            self.save(false, window, cx);
            if self.dirty() || self.asking || self.renaming {
                return;
            }
        }
        let Some(value) = self.pick_value(self.doc.shown.as_deref()) else { return };
        let appearance = (self.themes.resolve)(&self.doc.folded()).map_or(Appearance::Dark, |t| t.appearance);
        SettingsStore::update(cx, |s| picks::set_pick(s, value, appearance));
        cx.notify();
    }

    /// Whether Use with Match system is offered: a saved pair whose two appearances are not both
    /// picks yet.
    pub(super) fn can_follow_system(&self, cx: &App) -> bool {
        let (Some(light), Some(dark)) = (self.pick_value(Some(LIGHT)), self.pick_value(Some(DARK))) else {
            return false;
        };
        let s = settings(cx);
        self.doc.is_pair() && self.doc.file.id.is_some() && !(s.theme_light == light && s.theme == dark)
    }

    /// Use with Match system: the light pick is the pair's light appearance, the dark pick its dark one.
    pub(super) fn follow_system(&mut self, cx: &mut Context<Self>) {
        let (Some(light), Some(dark)) = (self.pick_value(Some(LIGHT)), self.pick_value(Some(DARK))) else {
            return;
        };
        SettingsStore::update(cx, |s| {
            s.theme_mode = ThemeMode::System;
            s.theme_light = light;
            s.theme = dark;
        });
        self.notice = Some(Notice::Done(studio_pair_used(self.doc.name()).into()));
        cx.notify();
    }
}
