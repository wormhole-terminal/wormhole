//! Versions: the earlier saves of a theme, newest first, the file as it is now on top and the
//! original file marked. Selecting a row (the arrows or the pointer) previews it; Restore loads it
//! as unsaved edits, so it can be undone or saved. Reading a save is capped and link-safe (core).

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::saved_versions::SavedVersion;
use wormhole_core::theme::tokens::ThemeFile;
use wormhole_text::keys::{
    STUDIO_VERSIONS_CURRENT, STUDIO_VERSIONS_DONE, STUDIO_VERSIONS_EMPTY, STUDIO_VERSIONS_FIRST,
    STUDIO_VERSIONS_JUST_NOW, STUDIO_VERSIONS_NOTE, STUDIO_VERSIONS_ORIGINAL, STUDIO_VERSIONS_RESTORE,
    STUDIO_VERSIONS_UNREADABLE, studio_versions_hours_ago, studio_versions_label,
    studio_versions_minutes_ago, studio_versions_title,
};
use wormhole_text::text;

use super::StudioWindow;
use super::document::history::StepLabel;
use super::document::{Subject, changed_tokens};
use super::modal::{Modal, card};
use super::names::display_date;
use crate::settings::UiColors;
use crate::style::{self, TextRole as _};

/// How many token names a row lists before "+n".
const NAMES_SHOWN: usize = 2;

/// One line of the panel.
pub(super) struct Row {
    /// The saved version; `None` for the file as it is now.
    version: Option<SavedVersion>,
    /// What the file says, or why it can't be read.
    file: Result<ThemeFile, String>,
    /// How it differs from the save before it.
    summary: String,
}

pub(super) struct Panel {
    rows: Vec<Row>,
    selected: usize,
}

impl Panel {
    #[cfg(test)]
    pub(super) fn row_count(&self) -> usize {
        self.rows.len()
    }
}

/// The tokens a row says it changed, `primary, git.branch +2`.
pub(super) fn summary_line(changed: &BTreeSet<String>) -> String {
    let mut names: Vec<&str> = changed.iter().map(String::as_str).take(NAMES_SHOWN).collect();
    if changed.len() > NAMES_SHOWN {
        let more = format!("+{}", changed.len() - NAMES_SHOWN);
        names.push(&more);
        return names[..NAMES_SHOWN].join(", ") + " " + &more;
    }
    names.join(", ")
}

/// When a save was made, in words that need no clock: "just now", "12 minutes ago", "3 hours ago",
/// then the day.
pub(super) fn when(saved: i64, now: i64) -> String {
    let ago = (now - saved).max(0);
    match ago {
        0..=59 => text(STUDIO_VERSIONS_JUST_NOW).to_string(),
        60..=3_599 => studio_versions_minutes_ago(ago / 60),
        3_600..=172_799 => studio_versions_hours_ago(ago / 3_600),
        _ => display_date(&super::names::day_of(saved)),
    }
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

impl Row {
    fn what(&self, now: i64) -> String {
        match &self.version {
            None => text(STUDIO_VERSIONS_CURRENT).to_string(),
            Some(v) => {
                let when = when(v.saved, now);
                if v.pinned { format!("{when} \u{b7} {}", text(STUDIO_VERSIONS_ORIGINAL)) } else { when }
            }
        }
    }
}

impl StudioWindow {
    /// Browse All Versions…: the file as it is now and the saves behind it.
    pub(super) fn open_versions(&mut self, cx: &mut Context<Self>) {
        let Subject::Own { path } = self.doc.subject.clone() else { return };
        let saved = self.doc.saved.clone();
        let mut rows = vec![Row { version: None, file: Ok(saved.clone()), summary: String::new() }];
        for version in (self.themes.versions)(&path, &saved) {
            let file = (self.themes.read_version)(&version);
            rows.push(Row { version: Some(version), file, summary: String::new() });
        }
        // Each save says what it changed from the one before it (the oldest: that it was the first).
        for ix in 0..rows.len() {
            let Ok(file) = &rows[ix].file else { continue };
            let older = rows.get(ix + 1).and_then(|r| r.file.as_ref().ok());
            let summary = match older {
                Some(older) => summary_line(&changed_tokens(older, file)),
                None => text(STUDIO_VERSIONS_FIRST).to_string(),
            };
            rows[ix].summary = summary;
        }
        self.modal = Some(Modal::Versions(Panel { rows, selected: 0 }));
        cx.notify();
    }

    /// Select a row: the gallery and the app show that save.
    pub(super) fn select_version(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(Modal::Versions(panel)) = self.modal.as_mut() else { return };
        if ix >= panel.rows.len() {
            return;
        }
        panel.selected = ix;
        let shown = self.doc.shown.clone();
        let file = match &panel.rows[ix].file {
            Ok(file) if ix > 0 => file.clone(),
            _ => self.doc.file.clone(),
        };
        let resolved = (self.themes.resolve)(&super::document::places::folded(&file, shown.as_deref())).ok();
        crate::settings::SettingsStore::preview_tokens(cx, resolved);
        cx.refresh_windows();
        cx.notify();
    }

    pub(super) fn move_version_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(Modal::Versions(panel)) = self.modal.as_ref() else { return };
        let last = panel.rows.len().saturating_sub(1);
        let ix = panel.selected.saturating_add_signed(delta).min(last);
        self.select_version(ix, cx);
    }

    /// Restore: the selected save becomes the edits, as one step to undo.
    pub(super) fn restore_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Modal::Versions(panel)) = self.modal.as_ref() else { return };
        let ix = panel.selected;
        let Some(Ok(file)) = panel.rows.get(ix).filter(|_| ix > 0).map(|r| r.file.clone()) else { return };
        self.modal = None;
        let shown = self.doc.shown.clone();
        if let Err(err) = self.commit(StepLabel::Restore, file, shown, window, cx) {
            self.field_error = Some(super::FieldError::from_resolve(&err));
        }
        cx.notify();
    }

    pub(super) fn render_versions(&self, panel: &Panel, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let now = now_secs();
        let rows =
            panel.rows.iter().enumerate().map(|(ix, row)| self.version_row(panel, ix, row, now, ui, cx));
        let body = div().flex().flex_col().gap_2().child(
            div()
                .id("studio-versions-rows")
                .flex()
                .flex_col()
                .max_h(px(360.0))
                .overflow_y_scroll()
                .children(rows),
        );
        let body = if panel.rows.len() == 1 {
            body.child(style::empty_note(ui, text(STUDIO_VERSIONS_EMPTY)))
        } else {
            body
        };
        card(
            ui,
            studio_versions_title(self.doc.name()),
            body,
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().text_caption().text_color(ui.faint).child(text(STUDIO_VERSIONS_NOTE)))
                .child(
                    Button::new("studio-versions-done")
                        .outline()
                        .label(text(STUDIO_VERSIONS_DONE))
                        .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                ),
        )
    }

    fn version_row(
        &self,
        panel: &Panel,
        ix: usize,
        row: &Row,
        now: i64,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = panel.selected == ix;
        let (what, readable) = (row.what(now), row.file.is_ok());
        let summary =
            if readable { row.summary.clone() } else { text(STUDIO_VERSIONS_UNREADABLE).to_string() };
        let label = studio_versions_label(&what, &summary);
        style::row(SharedString::from(format!("studio-version-{ix}")), ui, selected)
            .aria_label(label)
            .when(!readable, |d| d.text_color(ui.faint))
            .child(div().w(px(150.0)).flex_none().text_caption().child(what))
            .child(div().flex_1().min_w_0().truncate().text_caption().text_color(ui.muted).child(summary))
            .when(selected && ix > 0 && readable, |d| {
                d.child(
                    Button::new("studio-version-restore")
                        .small()
                        .primary()
                        .label(text(STUDIO_VERSIONS_RESTORE))
                        .on_click(cx.listener(|this, _, window, cx| this.restore_version(window, cx))),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| this.select_version(ix, cx)))
            .into_any_element()
    }

    /// Keys while a panel is open: the arrows walk the saves, Return restores, Esc leaves.
    pub(super) fn modal_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match (self.modal.as_ref(), key) {
            (None, _) => return false,
            (Some(_), "escape") => self.close_modal(cx),
            (Some(Modal::Versions(_)), "up") => self.move_version_selection(-1, cx),
            (Some(Modal::Versions(_)), "down") => self.move_version_selection(1, cx),
            (Some(Modal::Versions(_)), "home") => self.move_version_selection(isize::MIN / 2, cx),
            (Some(Modal::Versions(_)), "end") => self.move_version_selection(isize::MAX / 2, cx),
            (Some(Modal::Versions(_)), "enter") => self.restore_version(window, cx),
            (Some(Modal::Update(_)), "enter") => self.apply_update(window, cx),
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{summary_line, when};

    #[test]
    fn a_row_lists_a_few_tokens_and_counts_the_rest() {
        let set = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<BTreeSet<_>>();
        assert_eq!(summary_line(&set(&[])), "");
        assert_eq!(summary_line(&set(&["primary"])), "primary");
        assert_eq!(summary_line(&set(&["primary", "git.branch"])), "git.branch, primary");
        assert_eq!(summary_line(&set(&["a", "b", "c", "d"])), "a, b +2");
    }

    #[test]
    fn a_save_is_dated_in_words_that_need_no_clock() {
        let now = 1_800_000_000;
        assert_eq!(when(now - 10, now), "just now");
        assert_eq!(when(now - 120, now), "2 minutes ago");
        assert_eq!(when(now - 60, now), "1 minute ago");
        assert_eq!(when(now - 3 * 3_600, now), "3 hours ago");
        assert!(when(now - 10 * 86_400, now).contains("20"), "a date");
    }
}
