//! Update from the theme it was copied from: the values the source has changed since the copy,
//! listed old to new with the ones that are the theme's own marked as kept. Nothing is written
//! until Update, and then as one step to undo.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::appearance::as_variant_base;
use wormhole_core::theme::tokens::copy::{BaseChange, diff_base};
use wormhole_core::theme::tokens::slug::builtin_name;
use wormhole_core::theme::tokens::{BaseLayer, Color, ThemeFile};
use wormhole_text::keys::{
    STUDIO_UPDATE_APPLY, STUDIO_UPDATE_CANCEL, STUDIO_UPDATE_KEPT, studio_update_count, studio_update_title,
};
use wormhole_text::text;

use super::document::history::StepLabel;
use super::document::places;
use super::modal::{Modal, card};
use super::names::display_date;
use super::themes::Themes;
use super::{StudioWindow, color_chip};
use crate::settings::UiColors;
use crate::style::{self, TextRole as _};

/// What comparing the base with its source found, for the title row to say.
#[derive(Default)]
pub(super) struct Check {
    key: Option<(String, Option<String>, Option<String>)>,
    pub(super) state: State,
    changes: Vec<BaseChange>,
    fresh: Option<BaseLayer>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum State {
    /// The theme has no base (it is its own palette).
    #[default]
    NoBase,
    /// The source is gone.
    SourceMissing,
    UpToDate,
    Changed(usize),
}

impl Check {
    pub(super) fn run(file: &ThemeFile, shown: Option<&str>, themes: &Themes) -> Self {
        let Some(base) = places::base_under(file, shown) else { return Self::default() };
        let key = Some((base.from.clone(), base.copied.clone(), shown.map(str::to_string)));
        // A base made by Wormhole (the other appearance, generated from the first) was never
        // copied from its source: there is no copy to bring up to date.
        if base.copied.is_none() {
            return Self { key, ..Self::default() };
        }
        let Some(mut fresh) = (themes.fresh_base)(&base.from) else {
            return Self { key, state: State::SourceMissing, ..Self::default() };
        };
        // The base of an appearance of a pair is held without the accent (the theme's, shared) and
        // with the appearance it is for. A fresh copy is compared, and replaces it, in that form.
        let mut stored = base.clone();
        if places::is_pair(file) {
            as_variant_base(&mut stored, base.file.appearance);
            as_variant_base(&mut fresh, base.file.appearance);
        }
        let changes = diff_base(&stored, &fresh, &places::own_layers(file, shown));
        let state = if changes.is_empty() { State::UpToDate } else { State::Changed(changes.len()) };
        Self { key, state, changes, fresh: Some(fresh) }
    }
}

/// The sheet: what would change.
pub(super) struct Sheet {
    from: String,
    date: String,
    changes: Vec<BaseChange>,
    fresh: BaseLayer,
}

/// The name people know a base's source by.
pub(super) fn source_name(from: &str) -> String {
    builtin_name(from).map_or_else(|| from.to_string(), str::to_string)
}

impl StudioWindow {
    /// Compare the base with its source again, if it is another base than last time.
    pub(super) fn refresh_update_check(&mut self) {
        let base = places::base_under(&self.doc.file, self.doc.shown.as_deref());
        let key = base.map(|b| (b.from.clone(), b.copied.clone(), self.doc.shown.clone()));
        if key.is_some() && key == self.update_check.key {
            return;
        }
        self.update_check = Check::run(&self.doc.file, self.doc.shown.as_deref(), &self.themes);
    }

    /// Update from {source}…: the sheet, made from the source as it is now.
    pub(super) fn open_update_from(&mut self, cx: &mut Context<Self>) {
        self.update_check = Check::run(&self.doc.file, self.doc.shown.as_deref(), &self.themes);
        let (Some(base), Some(fresh)) =
            (places::base_under(&self.doc.file, self.doc.shown.as_deref()), self.update_check.fresh.clone())
        else {
            return;
        };
        if self.update_check.changes.is_empty() {
            return;
        }
        self.modal = Some(Modal::Update(Box::new(Sheet {
            from: source_name(&base.from),
            date: base.copied.as_deref().map(display_date).unwrap_or_default(),
            changes: self.update_check.changes.clone(),
            fresh,
        })));
        cx.notify();
    }

    /// Update: the base becomes the source as it is now; the theme's own values stay on top.
    pub(super) fn apply_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Modal::Update(sheet)) = self.modal.take() else { return };
        let mut file = self.doc.file.clone();
        places::set_base(&mut file, self.doc.shown.as_deref(), sheet.fresh);
        let shown = self.doc.shown.clone();
        self.update_check = Check::default();
        if let Err(err) = self.commit(StepLabel::UpdateFrom, file, shown, window, cx) {
            self.field_error = Some(super::FieldError::from_resolve(&err));
        }
        cx.notify();
    }

    pub(super) fn render_update_sheet(
        &self,
        sheet: &Sheet,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rows = sheet.changes.iter().map(|change| change_row(change, ui));
        let count = studio_update_count(i64::try_from(sheet.changes.len()).unwrap_or(i64::MAX), &sheet.date);
        card(
            ui,
            studio_update_title(&sheet.from),
            div().flex().flex_col().gap_3().child(div().text_body().text_color(ui.muted).child(count)).child(
                div()
                    .id("studio-update-rows")
                    .flex()
                    .flex_col()
                    .max_h(px(320.0))
                    .overflow_y_scroll()
                    .children(rows),
            ),
            div()
                .flex()
                .justify_end()
                .gap_2()
                .child(
                    Button::new("studio-update-cancel")
                        .outline()
                        .label(text(STUDIO_UPDATE_CANCEL))
                        .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                )
                .child(
                    Button::new("studio-update-apply")
                        .primary()
                        .label(text(STUDIO_UPDATE_APPLY))
                        .on_click(cx.listener(|this, _, window, cx| this.apply_update(window, cx))),
                ),
        )
    }
}

/// A swatch for a colour, otherwise the value as it is written (an expression, a number).
fn value_mark(value: Option<&str>, ui: UiColors) -> AnyElement {
    match value {
        None => div().flex_none().text_color(ui.faint).child("\u{2014}").into_any_element(),
        Some(v) => match Color::from_hex(v) {
            Ok(color) => div()
                .flex_none()
                .flex()
                .items_center()
                .gap_1()
                .child(color_chip(ui, color))
                .child(div().text_caption().text_color(ui.faint).child(v.to_string()))
                .into_any_element(),
            Err(_) => div()
                .min_w_0()
                .truncate()
                .text_caption()
                .text_color(ui.faint)
                .child(v.to_string())
                .into_any_element(),
        },
    }
}

fn change_row(change: &BaseChange, ui: UiColors) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .py_1()
        .child(div().w(px(150.0)).flex_none().truncate().text_caption().child(change.key.clone()))
        .child(value_mark(change.old.as_deref(), ui))
        .child(style::icon(gpui_kit::assets::IconName::ArrowRight).text_color(ui.faint))
        .child(value_mark(change.new.as_deref(), ui))
        .child(div().flex_1())
        .when(change.kept, |d| d.child(style::status_badge(ui, ui.faint, text(STUDIO_UPDATE_KEPT))))
}
