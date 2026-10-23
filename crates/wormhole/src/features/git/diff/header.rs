//! The diff view's title bar entry (where the file sits among its siblings)
//! and the card's header row: path, badges, counts and the view controls. With room for a split
//! view it is a 44 pt row with the layout switch and "esc close"; narrower it is a
//! 40 pt row, the switch hidden and a × to close.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Selectable, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::DiffView;
use super::render::DiffColors;
use crate::settings::{SettingsStore, UiColors};
use crate::settings::{settings_in, ui_in};
use crate::style::{self, Edges, TextRole};
use crate::terminal_view::TerminalView;
use crate::window::facts::WindowFacts;
use crate::window::features::git::changes::Changes;
use wormhole_core::git::{ChangeKind, Side};
use wormhole_core::view::limits::count_grouped;

/// The card's header beside a split view; a narrow one is a title row tall.
const HEADER_H: f32 = 44.0;
/// "esc close": a target this tall around its key cap.
const CLOSE_H: f32 = 24.0;
/// With the title row's own 4 pt, this puts the first chevron's glyph on the card's 16 pt
/// inset below it.
const TITLE_LEAD: f32 = 6.0;

impl Changes {
    /// Title bar while a diff is open: where the file sits among its siblings.
    pub(crate) fn render_diff_title(
        &mut self,
        facts: &WindowFacts<'_>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.follow(facts);
        let view = self.diff.as_ref()?;
        let ui = ui_in(&self.profile, cx);
        let position = view.siblings.iter().position(|f| f.path == view.file().path);
        let count = view.siblings.len();
        Some(
            div()
                .flex()
                .min_w_0()
                .items_center()
                .gap_1()
                .ml(px(TITLE_LEAD))
                .child(
                    style::icon_button("diff-prev", ui, IconName::ChevronLeft, "Previous file")
                        .in_title_row()
                        .on_click(cx.listener(|this, _, _, cx| this.step_diff(-1, cx))),
                )
                .child(
                    style::icon_button("diff-next", ui, IconName::ChevronRight, "Next file")
                        .in_title_row()
                        .on_click(cx.listener(|this, _, _, cx| this.step_diff(1, cx))),
                )
                .child(
                    div()
                        .flex()
                        .ml_1()
                        .items_center()
                        .gap_2()
                        .text_caption()
                        .text_color(ui.muted)
                        .children(position.map(|ix| format!("File {} of {count}", ix + 1)))
                        .children(view.context.clone().map(|c| div().text_color(ui.primary).child(c))),
                )
                .into_any_element(),
        )
    }

    /// The card's top row: the path, what happened to the file, +/− counts, the
    /// layout switch and close. `split_fits`: there is room for a split view, and
    /// with it for the full row.
    pub(super) fn render_diff_header(
        &self,
        view: &DiffView,
        ui: UiColors,
        colors: DiffColors,
        split_fits: bool,
        terminal: &Entity<TerminalView>,
        cx: &mut Context<Self>,
    ) -> Div {
        let (font_family, font_size) = crate::fonts::code_font_in(&self.profile, cx);
        let wrap = settings_in(&self.profile, cx).diff_wrap;
        let split = view.split(&self.profile, split_fits, cx);
        let (dir, name) = match view.file().path.rsplit_once('/') {
            Some((dir, name)) => (format!("{dir}/"), name.to_string()),
            None => (String::new(), view.file().path.clone()),
        };
        let diff = view.diff.as_deref();
        let renamed_from = diff.and_then(|d| d.renamed_from.clone()).or(view.file().original_path.clone());
        let mut badges: Vec<(Hsla, SharedString)> = Vec::new();
        match view.sides {
            (Side::Missing, _) => badges.push((ui.success_text, "New file".into())),
            (_, Side::Missing) => badges.push((ui.danger_text, "Deleted".into())),
            _ => {}
        }
        if renamed_from.is_some() || view.file().kind == ChangeKind::Renamed {
            badges.push((ui.primary, "Renamed".into()));
        }
        if let Some(sub) = &view.file().submodule {
            // What changed, in the row's words, where a file has "New file" or "Renamed".
            badges.push((ui.muted, sub.label(view.file().kind).into()));
        }
        if view.gone {
            // The diff stays as it was read; the status no longer lists the file.
            badges.push((ui.muted, "No longer changed".into()));
        }
        let binary = diff.is_some_and(|d| d.binary);
        if binary {
            badges.push((ui.muted, "Binary".into()));
        }
        if view.lfs() {
            badges.push((ui.muted, "LFS".into()));
        }
        if let Some((old, new)) = diff.and_then(|d| d.mode_change.clone()) {
            badges.push((ui.muted, format!("Mode {old} → {new}").into()));
        }
        if diff.is_some_and(|d| d.highlight_skipped) {
            badges.push((ui.muted, "No highlighting".into()));
        }
        let refresh_error = view.refresh_error.clone().map(|message| {
            div()
                .id("diff-refresh-error")
                .flex_none()
                .tooltip(move |window, cx| Tooltip::new(message.clone()).build(window, cx))
                .child(style::status_badge(ui, ui.warning_text, "Out of date"))
        });
        // A binary file has no lines to count, and an LFS pointer's lines say nothing.
        let pointers = diff.is_some_and(|d| d.lfs.is_some());
        // A submodule's commits have no lines to count, wrap or lay out in two columns.
        let lines = !binary && !pointers && view.file().submodule.is_none();
        let controls = controls(split_fits && lines, lines);
        let counts = controls.counts.then(|| {
            let (added, removed) = view.counts;
            (format!("+{}", count_grouped(added)), format!("−{}", count_grouped(removed)))
        });

        div()
            .flex()
            .flex_none()
            .items_center()
            .map(|d| {
                if split_fits {
                    d.h(px(HEADER_H)).gap_3().pl_4().pr_3()
                } else {
                    d.h(px(style::TITLE_ROW_H)).gap_2().pl_3().pr_1()
                }
            })
            .rule_bottom(ui)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            // Nothing of the path is drawn over the badges and counts after it
                            // (at 320: the name used to run on under them).
                            .overflow_hidden()
                            .items_center()
                            .gap_1()
                            .font_family(font_family.clone())
                            // Narrow, the path is the code's size, so more of it fits.
                            .map(|d| if split_fits { d.text_body() } else { d.text_size(font_size) })
                            .children(renamed_from.map(|from| {
                                div()
                                    .flex()
                                    .min_w_0()
                                    .items_center()
                                    .gap_1()
                                    .text_color(ui.muted)
                                    .child(
                                        div().min_w_0().text_ellipsis_start().overflow_hidden().child(from),
                                    )
                                    .child(style::icon(IconName::ArrowRight).xsmall())
                            }))
                            // The folder gives way first; the file name stays whole as long as
                            // the folder has anything left to give, then it is cut too.
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis_start()
                                    .text_color(ui.muted)
                                    .child(dir),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_shrink(NAME_SHRINK)
                                    .truncate()
                                    .text_color(ui.text)
                                    .child(name),
                            ),
                    )
                    .children(badges.into_iter().map(|(color, text)| style::status_badge(ui, color, text)))
                    .children(refresh_error),
            )
            .children(counts.map(|(added, removed)| {
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .font_family(font_family)
                    .map(|d| if split_fits { d.text_body() } else { d.text_caption() })
                    .child(div().text_color(colors.added).child(added))
                    .child(div().text_color(colors.removed).child(removed))
            }))
            .children(view.open_file(cx).map(|file| file.header_button(ui)))
            .when(controls.wrap, |d| {
                d.child(
                    style::icon_button("diff-wrap", ui, IconName::TextWrap, "Wrap long lines")
                        .in_title_row()
                        .selected(wrap)
                        .on_click(cx.listener(|_, _, _, cx| {
                            SettingsStore::update(cx, |s| s.diff_wrap = !s.diff_wrap);
                            cx.notify();
                        })),
                )
            })
            .map(|d| {
                if controls.switch {
                    d.child(
                        style::segmented(ui)
                            .child(
                                style::segment("diff-split", ui, "Split", split)
                                    .on_click(cx.listener(|this, _, _, cx| this.set_diff_split(true, cx))),
                            )
                            .child(
                                style::segment("diff-unified", ui, "Unified", !split)
                                    .on_click(cx.listener(|this, _, _, cx| this.set_diff_split(false, cx))),
                            ),
                    )
                    .child(
                        div()
                            .id("diff-close")
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap_1()
                            .h(px(CLOSE_H))
                            .px_1()
                            .rounded(ui.radius)
                            .cursor_pointer()
                            .text_caption()
                            .text_color(ui.muted)
                            .hover(|d| d.text_color(ui.text))
                            .on_click({
                                let terminal = terminal.clone();
                                cx.listener(move |this, _, window, cx| this.close_diff(&terminal, window, cx))
                            })
                            .child(style::keycap(ui, "esc"))
                            .child("close"),
                    )
                } else {
                    // No room for the switch: the diff is unified until there is.
                    d.child(
                        style::icon_button("diff-close", ui, IconName::X, "Close diff (esc)")
                            .in_title_row()
                            .on_click({
                                let terminal = terminal.clone();
                                cx.listener(move |this, _, window, cx| this.close_diff(&terminal, window, cx))
                            }),
                    )
                }
            })
    }
}

/// How much of the overflow the file name takes while its folder can still give way: next to
/// nothing, so the folder is gone before the name loses a letter, and the name is cut (at its
/// end) only once the folder is.
const NAME_SHRINK: f32 = 0.01;

/// Which controls the header shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Controls {
    /// The +/− counts: only over lines (a binary file has none, an LFS pointer's say nothing).
    counts: bool,
    /// Wrap long lines: the wide header keeps it on every card; the narrow one, with
    /// little room beside the name, only where there are lines to wrap.
    wrap: bool,
    /// Split / Unified and "esc close": only with room for a split view.
    switch: bool,
}

fn controls(split_fits: bool, lines: bool) -> Controls {
    Controls { counts: lines, wrap: lines || split_fits, switch: split_fits }
}

#[cfg(test)]
mod tests {
    use super::{Controls, controls};

    #[test]
    fn a_card_without_lines_drops_the_counts_and_narrow_the_wrap_button_too() {
        let all = Controls { counts: true, wrap: true, switch: true };
        assert_eq!(controls(true, true), all, "every control shows");
        assert_eq!(
            controls(true, false),
            Controls { counts: false, ..all },
            "a card without lines has no counts"
        );
        assert_eq!(
            controls(false, true),
            Controls { switch: false, ..all },
            "no switch without room for a split"
        );
        // The binary and LFS cards at 320: the name, the badge and the × alone.
        assert_eq!(controls(false, false), Controls { counts: false, wrap: false, switch: false });
    }
}
