//! The funnel in the Terminals header and what it opens: search, Group by, Show, Hide
//! idle, Density and Row details in one popover; the "Filtered: …" chip under the header
//! while a filter is on; the count of what it hides; and the empty list when nothing matches.
use gpui_kit::component::Selectable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::popover::Popover;
use wormhole_core::config::Settings;
use wormhole_core::sidebar::{Filter, Show, hidden_label};

use super::*;
use crate::select;
use crate::workspace::settings_page::{Section as SettingsSection, open_settings, update_settings};

/// What the popover shows, copied when the header is drawn: its content is built later, by
/// the popover, without the workspace at hand.
#[derive(Clone)]
struct Snapshot {
    grouping: Grouping,
    filter: Filter,
    idle: usize,
    compact: bool,
    branch: bool,
    changes: bool,
    notes: bool,
}

/// The popover's labels: 20 tall, the text at their foot.
const LABEL_H: f32 = 20.0;
/// The "Filtered" chip and its clear button.
const CHIP_H: f32 = 24.0;
const CHIP_ROW_H: f32 = 32.0;
/// The line under a filtered list ("8 terminals hidden · Show all").
const HIDDEN_ROW_H: f32 = 40.0;

impl Workspace {
    /// The funnel: tinted with a dot while the list isn't the default, the popover
    /// anchored under it.
    /// The funnel and its popover; `ring` while the title row's keys are on it.
    pub(in crate::workspace) fn filter_button(
        &self,
        idle: usize,
        ring: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !sidebar_filter_on(cx) {
            return None;
        }
        let ui = ui(cx);
        let s = settings(cx);
        let view = &self.sidebar.view;
        let changed = view.filter.is_active() || view.grouping != s.sidebar_group;
        let snapshot = Snapshot {
            grouping: view.grouping,
            filter: view.filter.clone(),
            idle,
            compact: s.compact_sidebar,
            branch: s.sidebar_branch,
            changes: s.sidebar_changes,
            notes: s.sidebar_notes,
        };
        let this = cx.weak_entity();
        let search = self.sidebar.search.clone();
        let on_open = this.clone();
        let popover = Popover::new("sidebar-filter")
            .anchor(Anchor::TopCenter)
            .offset(px(6.0))
            .open(self.sidebar.filter_open)
            // Typing goes straight into the search field.
            .track_focus(&search.read(cx).focus_handle(cx))
            .on_open_change(move |open, _, cx| {
                on_open
                    .update(cx, |this, cx| {
                        this.sidebar.filter_open = *open;
                        cx.notify();
                    })
                    .ok();
            })
            .trigger(
                style::icon_button("sidebar-filter", ui, IconName::Funnel, "Group and filter terminals")
                    .in_title_row()
                    .selected(changed)
                    .marked(changed)
                    .focus_ring(ring),
            );
        Some(
            select::menu_surface(popover, ui, px(style::FILTER_POPOVER_W), select::menu_bg(ui, cx))
                .p_3()
                .content(move |_, _, _| popover_content(&snapshot, &search, &this, ui))
                .into_any_element(),
        )
    }

    /// Clear the filter: everything shown, no search.
    pub(super) fn clear_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.view.filter = Filter::default();
        self.sidebar.search.update(cx, |input, cx| input.set_value("", window, cx));
        self.mark_dirty(cx);
    }

    pub(crate) fn set_show(&mut self, show: Show, cx: &mut Context<Self>) {
        if self.sidebar.view.filter.show != show {
            self.sidebar.view.filter.show = show;
            self.mark_dirty(cx);
        }
    }

    /// Set the search text (the field shows it too). Does nothing while the funnel is off.
    #[cfg(feature = "devctl")]
    pub(crate) fn set_search(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !sidebar_filter_on(cx) {
            return;
        }
        self.sidebar.search.update(cx, |input, cx| input.set_value(text.to_string(), window, cx));
        self.sidebar.view.filter.search = text.to_string();
        self.mark_dirty(cx);
    }

    /// The chip under the header while a filter is on: what it keeps, and ✕ to clear it.
    pub(super) fn filter_chip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ui = ui(cx);
        let text = self.sidebar.view.as_seen(sidebar_filter_on(cx)).filter.chip()?;
        Some(
            div()
                .flex_none()
                .h(px(CHIP_ROW_H))
                .flex()
                .items_center()
                .px_4()
                .child(
                    style::tint_chip(ui, None, text).h(px(CHIP_H)).min_w_0().child(
                        style::close_x("clear-filter", ui, style::SelectedFill::Tint)
                            .tooltip(|window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new("Clear filter").build(window, cx)
                            })
                            .on_click(cx.listener(|this, _, window, cx| this.clear_filter(window, cx))),
                    ),
                )
                .into_any_element(),
        )
    }

    /// Under a filtered list: how many terminals it hides, and the way back.
    pub(super) fn hidden_line(&self, hidden: usize, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        div()
            .flex_none()
            .rule_top(ui)
            .h(px(HIDDEN_ROW_H))
            .flex()
            .items_center()
            .gap_2()
            .pl(px(style::SIDEBAR_INSET + style::ITEM_LEAD_W + 8.0))
            .pr_4()
            .text_caption()
            .child(div().min_w_0().truncate().text_color(ui.faint).child(hidden_label(hidden)))
            .child(
                link_button("show-all", "Show all", ui)
                    .on_click(cx.listener(|this, _, window, cx| this.clear_filter(window, cx))),
            )
            .into_any_element()
    }

    /// The list when the filter keeps nothing: say so, count what's hidden, offer the
    /// way out.
    pub(super) fn no_match(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let why = self.sidebar.view.filter.nothing_matches(self.sessions.len());
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .px(px(style::SIDEBAR_INSET + style::ITEM_LEAD_W + 8.0))
            .text_center()
            .child(Icon::new(IconName::Funnel).with_size(px(style::EMPTY_GLYPH)).text_color(ui.faint))
            .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child("No terminals match"))
            .child(div().text_caption().text_color(ui.muted).child(why))
            .child(
                Button::new("clear-filter-empty")
                    .label("Clear filter")
                    .small()
                    .outline()
                    .mt_1()
                    .on_click(cx.listener(|this, _, window, cx| this.clear_filter(window, cx))),
            )
            .into_any_element()
    }
}

fn popover_content(
    snapshot: &Snapshot,
    search: &Entity<InputState>,
    this: &WeakEntity<Workspace>,
    ui: UiColors,
) -> AnyElement {
    let update = |f: fn(&mut Workspace, &mut Window, &mut Context<Workspace>)| {
        let this = this.clone();
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            this.update(cx, |ws, cx| f(ws, window, cx)).ok();
        }
    };
    let grouping = |label: &'static str, grouping: Grouping| {
        let this = this.clone();
        style::segment(
            SharedString::from(format!("group-by-{label}")),
            ui,
            label,
            snapshot.grouping == grouping,
        )
        .flex_1()
        .justify_center()
        .on_click(move |_, _, cx| {
            this.update(cx, |ws, cx| ws.set_grouping(grouping, cx)).ok();
        })
    };
    let show = |show: Show| {
        let this = this.clone();
        style::segment(
            SharedString::from(format!("show-{}", show.label())),
            ui,
            show.label(),
            snapshot.filter.show == show,
        )
        .flex_1()
        .justify_center()
        .on_click(move |_, _, cx| {
            this.update(cx, |ws, cx| ws.set_show(show, cx)).ok();
        })
    };
    let density = |label: &'static str, compact: bool| {
        style::segment(SharedString::from(format!("density-{label}")), ui, label, snapshot.compact == compact)
            .flex_1()
            .justify_center()
            .on_click(move |_, _, cx| update_settings(cx, |s| s.compact_sidebar = compact))
    };
    let detail = |id: &'static str,
                  label: &'static str,
                  hint: Option<&'static str>,
                  on: bool,
                  flip: fn(&mut Settings)| {
        check_row(id, label, hint, on, ui).on_click(move |_, _, cx| update_settings(cx, flip))
    };
    let hide_idle = {
        let this = this.clone();
        check_row("hide-idle", "Hide idle terminals", None, snapshot.filter.hide_idle, ui)
            .child(
                div()
                    .flex_none()
                    .text_caption()
                    .text_color(ui.faint)
                    .child(format!("{} idle", snapshot.idle)),
            )
            .on_click(move |_, _, cx| {
                this.update(cx, |ws, cx| {
                    ws.sidebar.view.filter.hide_idle ^= true;
                    ws.mark_dirty(cx);
                })
                .ok();
            })
    };

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(Input::new(search).small().prefix(style::icon(IconName::Search).text_color(ui.muted)))
        .child(group(ui, "Group by").child(
            segments(ui).child(grouping("Folder", Grouping::Folder)).child(grouping("None", Grouping::None)),
        ))
        .child(group(ui, "Show").child(segments(ui).children(Show::ALL.map(show))).child(hide_idle))
        .child(separator(ui))
        .child(
            group(ui, "Density")
                .child(segments(ui).child(density("Comfortable", false)).child(density("Compact", true)))
                .child(note(ui, "Compact: one line per terminal, status on the right.")),
        )
        .child(
            group(ui, "Row details")
                .child(detail("detail-branch", "Branch", Some("on folder bands"), snapshot.branch, |s| {
                    s.sidebar_branch ^= true
                }))
                .child(detail("detail-changes", "Changed files", Some("±N"), snapshot.changes, |s| {
                    s.sidebar_changes ^= true
                }))
                .child(detail("detail-notes", "Notes and tasks", None, snapshot.notes, |s| {
                    s.sidebar_notes ^= true
                })),
        )
        .child(separator(ui))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    link_button("reset-sidebar", "Reset to defaults", ui)
                        .text_color(ui.muted)
                        .on_click(update(reset)),
                )
                .child(
                    link_button("sidebar-settings", "Terminals & sidebar settings…", ui)
                        .on_click(|_, _, cx| open_settings(Some(SettingsSection::Sessions), cx)),
                ),
        )
        .into_any_element()
}

/// "Reset to defaults": the grouping new windows get, no filter, and the row settings back
/// to theirs.
fn reset(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let defaults = Settings::default();
    update_settings(cx, |s| {
        s.compact_sidebar = defaults.compact_sidebar;
        s.sidebar_branch = defaults.sidebar_branch;
        s.sidebar_changes = defaults.sidebar_changes;
        s.sidebar_notes = defaults.sidebar_notes;
    });
    let grouping = settings(cx).sidebar_group;
    ws.set_grouping(grouping, cx);
    ws.clear_filter(window, cx);
}

/// A titled part of the popover.
#[track_caller]
fn group(ui: UiColors, title: &'static str) -> Div {
    div().flex().flex_col().gap_1().child(
        div()
            .h(px(LABEL_H))
            .flex()
            .items_end()
            .text_caption()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(ui.muted)
            .child(title),
    )
}

/// A segmented control across the popover's width.
#[track_caller]
fn segments(ui: UiColors) -> Div {
    style::segmented(ui).w_full()
}

#[track_caller]
fn note(ui: UiColors, text: &'static str) -> Div {
    div().text_caption().text_color(ui.faint).child(text)
}

#[track_caller]
fn separator(ui: UiColors) -> Div {
    div().h(ui.border_width).bg(ui.border).mx(px(-12.0))
}

/// A setting that is on or off: a check in the lead slot when on, the label, a faint hint.
#[track_caller]
fn check_row(
    id: &'static str,
    label: &'static str,
    hint: Option<&'static str>,
    on: bool,
    ui: UiColors,
) -> Stateful<Div> {
    style::row(id, ui, false)
        .mx(px(-8.0))
        .child(style::lead(div().when(on, |d| d.child(style::icon(IconName::Check).text_color(ui.primary)))))
        .child(div().flex_1().min_w_0().truncate().child(label))
        .children(hint.map(|hint| div().flex_none().text_caption().text_color(ui.faint).child(hint)))
}

/// A text button in the primary's color ("Show all", the settings link).
#[track_caller]
fn link_button(id: &'static str, label: &'static str, ui: UiColors) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(px(style::CONTROL_H_SM))
        .px_1()
        .rounded(ui.radius)
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.selected_text)
        .cursor_pointer()
        .hover(|d| d.bg(ui.hover))
        .child(label)
}
