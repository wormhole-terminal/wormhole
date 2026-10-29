//! Changing the file as one step: a new file (and appearance) is resolved first, kept as a step to
//! undo, and previewed. The few edits that are more than one value (moving a value between both
//! appearances and one, adding an appearance, switching which one is shown) are made here.

use gpui_kit::*;
use wormhole_core::theme::tokens::appearance::{DARK, LIGHT};
use wormhole_core::theme::tokens::{Appearance, ThemeFile};
use wormhole_text::keys::{
    STUDIO_ADD_FAILED, STUDIO_APPEARANCE_DARK, STUDIO_APPEARANCE_LIGHT, studio_moved_kept,
};
use wormhole_text::text;

use super::document::history::StepLabel;
use super::document::places;
use super::names::today;
use super::save::Notice;
use super::{StudioWindow, number};
use crate::settings::SettingsStore;

/// The appearance's name as people read it.
pub(super) fn appearance_word(name: &str) -> &'static str {
    if name == LIGHT { text(STUDIO_APPEARANCE_LIGHT) } else { text(STUDIO_APPEARANCE_DARK) }
}

impl StudioWindow {
    /// The file changed: the rows are made again, and what is derived from the file is checked
    /// again.
    pub(super) fn touched(&mut self) {
        self.revision += 1;
        self.refresh_update_check();
    }

    /// Show `tokens` everywhere (the whole app follows while the Studio is open).
    pub(super) fn preview_file(&self, file: &ThemeFile, cx: &mut App) {
        let resolved = (self.themes.resolve)(&places::folded(file, self.doc.shown.as_deref())).ok();
        SettingsStore::preview_tokens(cx, resolved);
        cx.refresh_windows();
    }

    /// `file` (with `shown` as the appearance being edited) becomes the file, as one step to undo.
    /// A file that does not resolve changes nothing.
    pub(super) fn commit(
        &mut self,
        label: StepLabel,
        file: ThemeFile,
        shown: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let resolved = (self.themes.resolve)(&places::folded(&file, shown.as_deref()))?;
        self.record_step(label, false, cx);
        self.end_gesture(cx);
        self.doc.file = file;
        self.doc.shown = shown;
        self.doc.settle_shown();
        self.touched();
        self.field_error = None;
        self.notice = None;
        SettingsStore::preview_tokens(cx, Some(resolved));
        cx.refresh_windows();
        self.publish_edit_titles(cx);
        let name = self.doc.name().to_string();
        self.name.update(cx, |input, cx| input.set_value(name, window, cx));
        if let Some(token) = self.selected.clone() {
            self.show_token(&token, true, window, cx);
        }
        cx.notify();
        Ok(())
    }

    /// Light | Dark: edit the other appearance. A view change, not a step to undo.
    pub(super) fn set_shown(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.doc.shown.as_deref() == Some(name) || !self.doc.file.variants.iter().any(|v| v.name == name) {
            return;
        }
        self.doc.shown = Some(name.to_string());
        self.touched();
        self.preview_file(&self.doc.file, cx);
        if let Some(token) = self.selected.clone() {
            self.show_token(&token, true, window, cx);
        }
        cx.notify();
    }

    /// The chip of a row: the value moves between both appearances and the shown one.
    pub(super) fn toggle_reach(&mut self, token: &str, window: &mut Window, cx: &mut Context<Self>) {
        let tokens = cx.global::<SettingsStore>().tokens.clone();
        let current = tokens
            .metrics
            .get(token)
            .filter(|_| super::is_metric(token))
            .map(number)
            .or_else(|| tokens.get(token).map(|c| c.to_hex()))
            .unwrap_or_default();
        let shown = self.doc.shown.clone();
        let mut file = self.doc.file.clone();
        let moved = match places::move_value(&mut file, shown.as_deref(), token, &current) {
            Ok(moved) => moved,
            Err(err) => {
                self.field_error = Some(super::FieldError::plain(err));
                cx.notify();
                return;
            }
        };
        if let Err(err) = self.commit(StepLabel::Move(token.to_string()), file, shown.clone(), window, cx) {
            self.field_error = Some(super::FieldError::from_resolve(&err));
            cx.notify();
            return;
        }
        if let Some((gone, value)) = moved.removed.first() {
            let kept = shown.as_deref().map_or("", appearance_word);
            self.notice =
                Some(Notice::Moved(studio_moved_kept(kept, token, appearance_word(gone), value).into()));
        }
        cx.notify();
    }

    /// Add Dark / Add Light Appearance: the theme becomes a pair.
    pub(super) fn add_appearance(&mut self, to: Appearance, window: &mut Window, cx: &mut Context<Self>) {
        if self.asking || !self.doc.file.variants.is_empty() || self.doc.is_newer() {
            return;
        }
        let made = self.pair_of(to);
        match made {
            Ok(pair) => {
                let shown = Some(if to == Appearance::Light { LIGHT } else { DARK }.to_string());
                if let Err(err) = self.commit(StepLabel::AddAppearance, pair, shown, window, cx) {
                    tracing::error!("adding an appearance failed: {err}");
                    self.notice = Some(Notice::Failed(text(STUDIO_ADD_FAILED).into()));
                }
            }
            Err(err) => {
                tracing::error!("adding an appearance failed: {err}");
                self.notice = Some(Notice::Failed(text(STUDIO_ADD_FAILED).into()));
            }
        }
        cx.notify();
    }

    /// The theme as a pair with the appearance `to` added. A theme in the older format is converted
    /// first (the pair is in the new one).
    fn pair_of(&self, to: Appearance) -> Result<ThemeFile, String> {
        let single = if self.doc.file.version.is_some() {
            self.doc.file.clone()
        } else {
            let path = match &self.doc.subject {
                super::document::Subject::Own { path } => Some(path.as_path()),
                _ => None,
            };
            let id = (self.themes.new_id)(self.doc.name(), path);
            super::convert::converted_safely(&self.themes, &self.doc.file, &id)?
        };
        let have = (self.themes.resolve)(&single)?;
        wormhole_core::theme::tokens::appearance::add_appearance(&single, &have, to, &|_| None, &today())
    }

    /// Which appearance Add Appearance would add, or `None` once the theme has both.
    pub(super) fn missing_appearance(&self) -> Option<Appearance> {
        if self.doc.file.variants.is_empty() {
            let have = (self.themes.resolve)(&self.doc.folded()).ok()?.appearance;
            return Some(if have == Appearance::Light { Appearance::Dark } else { Appearance::Light });
        }
        None
    }
}
