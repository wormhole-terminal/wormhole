//! The Changed page: every setting that differs from its default, each in the control of its own page and
//! under the name of its page, with what it is by default beside its name and a reset arrow of its own. At
//! the top, Copy as JSON (what differs, as a snippet for `config.json`) and Reset All…, which asks first.
//!
//! The rows are the registry's: the same renderer draws them here and on their pages, with their dot,
//! reset, key and right-click menu, so a change made here is the change made there. Which rows differ is
//! core's (`Values::changed_rows`); this page only lays them out. The nav lists the page only while one
//! does, with their count.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, ChangedRow, SectionId as Section, Values};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::SettingsWindow;
use super::nav::page_tile;
use super::registry::{Control, Current, PageState, Setting, control};
use crate::dialog::{self, DestructiveQuestion};
use crate::platform::clipboard;
use crate::settings::{Settings, SettingsStore, UiColors, settings, update_settings};
use crate::style::{self, RowTile, TileRowParts};

/// How long Copy as JSON says Copied.
const COPIED_FOR: Duration = Duration::from_millis(1500);

/// How many settings differ, for the nav's badge: the rows the page would list.
pub(super) fn differing(cx: &App) -> usize {
    let store = cx.global::<SettingsStore>();
    Values::of(store.user_settings()).changed_rows(&store.settings).len()
}

impl SettingsWindow {
    /// The page: the summary with its two buttons, then the rows of each page that has any.
    pub(super) fn render_changed(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = PageState::new(&self.front, cx);
        let ui = page.ui;
        let rows = page.values.changed_rows(settings(cx));
        let mut column = div().flex().flex_col().gap_6().child(self.changed_summary(ui, rows.len(), cx));
        let mut start = 0;
        while start < rows.len() {
            let section = rows[start].section;
            let end =
                rows[start..].iter().position(|row| row.section != section).map_or(rows.len(), |n| start + n);
            let mut list = Vec::new();
            for row in &rows[start..end] {
                let Some(setting) =
                    control(row.entry.id).filter(|_| self.drawn(row.entry, settings(cx)).is_some())
                else {
                    continue;
                };
                let note = default_note(row, setting, &page.values);
                list.push(self.render_entry(row.entry, setting, &page, None, Some(note), cx));
                if let Some(below) = setting.below.and_then(|below| below(self, cx)) {
                    list.push(div().pb_2().child(below).into_any_element());
                }
            }
            column = column.child(
                div()
                    .flex()
                    .flex_col()
                    .child(style::setting_group_head(ui, section.title()))
                    .child(style::group_card(ui, list)),
            );
            start = end;
        }
        column.into_any_element()
    }

    /// "5 settings changed" with the two buttons that act on all of them; with none, the page says so.
    fn changed_summary(&self, ui: UiColors, count: usize, cx: &mut Context<Self>) -> AnyElement {
        let tile = RowTile::Icon { fill: page_tile(Section::Changed, cx), icon: IconName::RotateCcw };
        let parts = if count == 0 {
            let mut parts = TileRowParts::new(tile, text(SETTINGS_CHANGED_NONE));
            parts.hint = Some(text(SETTINGS_CHANGED_NONE_DETAIL).into());
            parts
        } else {
            let mut parts = TileRowParts::new(tile, settings_changed_summary(whole(count)));
            parts.hint = Some(text(SETTINGS_CHANGED_SUMMARY_DETAIL).into());
            parts.controls = vec![
                style::form_button("changed-copy")
                    .outline()
                    .label(if self.changed_copied {
                        text(SETTINGS_CHANGED_COPIED)
                    } else {
                        text(SETTINGS_CHANGED_COPY)
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.copy_changed(cx)))
                    .into_any_element(),
                style::form_button("changed-reset-all")
                    .danger()
                    .outline()
                    .label(text(SETTINGS_CHANGED_RESET_ALL))
                    .on_click(cx.listener(|this, _, window, cx| this.ask_reset_changed(window, cx)))
                    .into_any_element(),
            ];
            parts
        };
        style::group_card(ui, [style::tile_row("changed-summary", ui, parts, cx).into_any_element()])
            .into_any_element()
    }

    /// Copy what differs, as the snippet a person pastes into `config.json`, and say so for a moment.
    fn copy_changed(&mut self, cx: &mut Context<Self>) {
        let store = cx.global::<SettingsStore>();
        let values = Values::of(store.user_settings());
        let rows = values.changed_rows(&store.settings);
        clipboard::write(values.json(&values.changed_keys_of(&rows)), cx);
        self.changed_copied = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            this.update(cx, |this, cx| {
                this.changed_copied = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Reset All…: ask, then put every listed setting back.
    fn ask_reset_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let count = differing(cx);
        if count == 0 {
            return;
        }
        ask_reset(window, cx, count, |cx| {
            // What differs when the answer comes, not when it was asked.
            let store = cx.global::<SettingsStore>();
            let values = Values::of(store.user_settings());
            let rows = values.changed_rows(&store.settings);
            let keys = values.changed_keys_of(&rows);
            update_settings(cx, |s| *s = settings_index::reset(s, &keys));
        });
    }
}

/// The question before settings go back to their defaults, `count` of them: the system's alert, the
/// destructive button red and Cancel the default. `reset` runs only for Reset. One way to ask, for Advanced's
/// "Reset to defaults…" and this page's Reset All….
pub(in crate::settings_window) fn ask_reset(
    window: &mut Window,
    cx: &mut App,
    count: usize,
    reset: impl FnOnce(&mut App) + 'static,
) {
    dialog::ask_destructive(
        window,
        cx,
        DestructiveQuestion {
            title: settings_changed_question(whole(count)).into(),
            detail: text(SETTINGS_CHANGED_QUESTION_DETAIL).into(),
            confirm: text(SETTINGS_CHANGED_QUESTION_CONFIRM),
        },
        move |_, cx| reset(cx),
    );
}

/// The line under a changed row's name: its group, then what the setting is by default, in the words of
/// its control.
fn default_note(row: &ChangedRow, setting: &Setting, values: &Values) -> SharedString {
    let value = default_words(row, setting, values);
    match row.group.title() {
        "" => settings_changed_default(&value),
        group => settings_changed_default_in_group(group, &value),
    }
    .into()
}

/// The default of a row, in its control's own words where it has them (a menu's option, a switch's On),
/// else in plain words from the defaults of its keys.
pub(super) fn default_words(row: &ChangedRow, setting: &Setting, values: &Values) -> String {
    let defaults = Settings::default();
    if let Some(words) = setting.default_words {
        return words().to_string();
    }
    match &setting.control {
        Control::Recorder(recorder) => {
            super::recorder::default_keys(recorder).unwrap_or_else(|| values.default_words(row.entry.keys))
        }
        Control::Menu(menu) => match menu.current(&defaults) {
            Current::At(at) => menu.options[at].label.words().to_string(),
            Current::Other(words) => words,
        },
        _ => values.default_words(row.entry.keys),
    }
}
