//! The keyboard-shortcuts sheet: Help → Keyboard Shortcuts, ⌥⌘/ (⌘/ is the status bar's),
//! and from the command palette. Core's `keymap::sheet` is what it lists, the action table seen
//! through its sections, so it shows the keys the menus show; a trusted project's own keys get a
//! block of their own. Typing filters by label or key ("⌘W" finds Close terminal); Esc clears, then
//! closes, and the keyboard goes back where it was.

use gpui_kit::component::input::{Input, InputEvent, InputState};
use wormhole_core::keymap::sheet::{self, Filtered, Hit, Keys, Row};
use wormhole_core::keyspec::KeySpec;
use wormhole_core::project::Feature;

use super::app::{AppState, OpenWindow, app};
use super::overlay::{OverlayView, scrim};
use super::*;

/// How far below the window's top the sheet opens, its widest, and the room left round it.
const TOP: f32 = 56.0;
const WIDTH: f32 = 1120.0;
const MARGIN: f32 = 24.0;
/// The least the sheet shrinks to while the filter leaves fewer columns (its filtered state):
/// the header's title, filter and key cap need this much.
const MIN_WIDTH: f32 = 520.0;
/// The header's height, and the filter field's width in it.
const HEADER_H: f32 = 56.0;
const FILTER_W: f32 = 240.0;
/// The footer's height.
const FOOTER_H: f32 = 40.0;
/// The space between columns, and the least width a column needs (fewer columns below it).
const COLUMN_GAP: f32 = 40.0;
const COLUMN_MIN: f32 = 280.0;

/// A trusted project's keys, for the block under the sections.
struct ProjectKeys {
    /// "terminal/.wormhole/actions.json".
    name: String,
    actions: Vec<(String, KeySpec)>,
}

pub(crate) struct ShortcutSheet {
    workspace: WeakEntity<Workspace>,
    filter: Entity<InputState>,
    sections: Vec<sheet::Section>,
    project: Option<ProjectKeys>,
    /// The global hotkey, as set (shown in the footer).
    hotkey: Option<String>,
    _subscriptions: Vec<Subscription>,
}

/// ⌥⌘/ (and Help → Keyboard Shortcuts, and the palette): open the sheet over the window in front,
/// or close it there. Runs outside any window's update.
pub(crate) fn toggle(cx: &mut App) {
    let Some(front) = AppState::front_window(cx) else { return };
    match front.workspace.read_with(cx, |ws, _| (ws.sheet_open(), ws.first_run.is_open())) {
        Ok((_, true)) | Err(_) => {}
        Ok((true, false)) => {
            AppState::with_workspace(&front, cx, |ws, window, cx| ws.close_overlay(window, cx));
        }
        Ok((false, false)) => open_in(&front, "", cx),
    }
}

/// Open the sheet over `front` with `query` in its filter.
pub(crate) fn open_in(front: &OpenWindow, query: &str, cx: &mut App) {
    let hotkey = settings(cx).hotkey.trim().parse::<KeySpec>().ok().map(|k| k.glyphs());
    let query = query.to_string();
    AppState::with_workspace(front, cx, move |ws, window, cx| {
        crate::background::activate_window(window, cx);
        let project = ws.project_keys(cx);
        let return_focus = ws.end_overlay(window, cx);
        let workspace = cx.weak_entity();
        let sheet = cx.new(|cx| ShortcutSheet::new(workspace, project, hotkey, &query, window, cx));
        ws.show_overlay(OverlayView::Sheet(sheet), return_focus, cx);
    });
}

impl Workspace {
    /// The current terminal's project keys, once it is trusted (an untrusted project shows
    /// nothing here).
    fn project_keys(&self, cx: &App) -> Option<ProjectKeys> {
        let hub = &app(cx).hub;
        let project = self.active_project(cx).filter(|p| hub.enabled(p, Feature::Actions))?;
        let actions: Vec<(String, KeySpec)> =
            project.actions().iter().filter_map(|a| Some((a.title.clone(), a.key_spec()?))).collect();
        let folder = project.folder.root.file_name()?.to_string_lossy().into_owned();
        (!actions.is_empty())
            .then(|| ProjectKeys { name: format!("{folder}/.wormhole/actions.json"), actions })
    }
}

impl ShortcutSheet {
    fn new(
        workspace: WeakEntity<Workspace>,
        project: Option<ProjectKeys>,
        hotkey: Option<String>,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter shortcuts"));
        filter.update(cx, |filter, cx| filter.set_value(query.to_string(), window, cx));
        window.focus(&filter.focus_handle(cx), cx);
        let handle = window.window_handle().window_id();
        let sheet = cx.weak_entity();
        // Ahead of the field, which would keep Esc for itself.
        let keys = cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != handle || event.keystroke.key != "escape" {
                return;
            }
            if sheet.update(cx, |sheet, cx| sheet.escape(window, cx)).is_ok() {
                cx.stop_propagation();
            }
        });
        let subscriptions = vec![
            keys,
            cx.subscribe_in(&filter, window, |_, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    cx.notify();
                }
            }),
        ];
        Self {
            workspace,
            filter,
            sections: sheet::sections(),
            project,
            hotkey,
            _subscriptions: subscriptions,
        }
    }

    /// Esc: the filter first, then the sheet.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter.read(cx).value().is_empty() {
            let _ = self.workspace.update(cx, |ws, cx| ws.close_overlay(window, cx));
        } else {
            self.filter.update(cx, |filter, cx| filter.set_value("", window, cx));
        }
    }

    /// What the filter leaves of the sheet (sections and the project's block), and the counts.
    fn filtered(&self, query: &str) -> (Filtered, Vec<(&str, &KeySpec, Hit)>) {
        let mut found = sheet::filter(&self.sections, query);
        let project: Vec<(&str, &KeySpec, Hit)> = self
            .project
            .iter()
            .flat_map(|p| &p.actions)
            .filter_map(|(title, key)| {
                sheet::matches(query, title, std::slice::from_ref(key)).map(|hit| (title.as_str(), key, hit))
            })
            .collect();
        found.total += self.project.as_ref().map_or(0, |p| p.actions.len());
        found.shown += project.len();
        (found, project)
    }
}

/// One shortcut: its label (what the filter found marked) and its keys at the right.
fn shortcut(
    ui: UiColors,
    label: SharedString,
    marks: Option<std::ops::Range<usize>>,
    keys: Vec<AnyElement>,
) -> Div {
    let marks: Vec<_> = marks.into_iter().collect();
    div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::ROW_H))
        .min_w_0()
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_label()
                .font_weight(FontWeight::NORMAL)
                .child(style::marked_text(ui, label, &marks, false)),
        )
        .child(div().flex_1())
        .child(div().flex().flex_none().items_center().gap_1().children(keys))
}

/// A row's keys as key caps: each, or a range's first and last with "…" between.
fn key_caps(ui: UiColors, row: &Row) -> Vec<AnyElement> {
    let cap = |key: &KeySpec| style::keycap(ui, key.glyphs()).into_any_element();
    match row.style {
        Keys::Each => row.keys.iter().map(cap).collect(),
        Keys::Range => {
            let mut caps: Vec<AnyElement> = row.keys.first().map(cap).into_iter().collect();
            if let Some(last) = row.keys.last().filter(|_| row.keys.len() > 1) {
                caps.push(div().text_caption().text_color(ui.faint).child("…").into_any_element());
                caps.push(cap(last));
            }
            caps
        }
    }
}

impl Render for ShortcutSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let viewport = window.viewport_size();
        let widest = WIDTH.min(f32::from(viewport.width) - 2.0 * style::SIDEBAR_INSET);
        let max_h = f32::from(viewport.height) - TOP - MARGIN;
        let query = self.filter.read(cx).value().to_string();
        let (found, project) = self.filtered(&query);
        // As many columns as there is room for; a filter that leaves fewer sections leaves
        // fewer columns, and the sheet shrinks to them, each keeping its full width.
        let fit = |width: f32| {
            let inner = width - 2.0 * MARGIN;
            (1..=3).rev().find(|&n| n == 1 || inner >= n as f32 * COLUMN_MIN + (n - 1) as f32 * COLUMN_GAP)
        };
        let full_columns = fit(widest).unwrap_or(1);
        let column_w = (widest - 2.0 * MARGIN - (full_columns - 1) as f32 * COLUMN_GAP) / full_columns as f32;
        let columns = if query.trim().is_empty() {
            full_columns
        } else {
            full_columns.min(found.sections.len().max(1))
        };
        let width = if columns == full_columns {
            widest
        } else {
            (2.0 * MARGIN + columns as f32 * column_w + (columns - 1) as f32 * COLUMN_GAP)
                .max(MIN_WIDTH)
                .min(widest)
        };
        let inner = width - 2.0 * MARGIN;
        let heights: Vec<usize> = found.sections.iter().map(|(_, rows)| rows.len() + 1).collect();
        let section = |(title, rows): &(&'static str, Vec<(Row, Hit)>)| {
            div().flex().flex_col().child(style::palette_label(ui, *title, None).px_0()).children(
                rows.iter()
                    .map(|(row, hit)| shortcut(ui, row.label.into(), hit.label.clone(), key_caps(ui, row))),
            )
        };
        let runs = sheet::columns(&heights, columns);
        let body =
            div().flex().gap(px(COLUMN_GAP)).px(px(MARGIN)).pt_2().pb_4().children((0..columns).map(|n| {
                let sections = runs.get(n).map(|run| &found.sections[run.clone()]).unwrap_or_default();
                div().flex_1().min_w_0().flex().flex_col().gap_4().children(sections.iter().map(section))
            }));
        let project_block = self.project.as_ref().filter(|_| !project.is_empty()).map(|keys| {
            div()
                .flex()
                .flex_col()
                .mx(px(MARGIN))
                .mb_4()
                .rule_top(ui)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(px(style::QP_ROW_H))
                        .text_label()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ui.muted)
                        .child(style::icon(IconName::Folder))
                        .child(format!("This project · {}", keys.name))
                        .child(div().flex_1())
                        .child(style::status_badge(ui, ui.success_text, "Trusted")),
                )
                .child(div().flex().flex_wrap().gap_x(px(COLUMN_GAP)).children(project.iter().map(
                    |(title, key, hit)| {
                        let cap = vec![style::keycap(ui, key.glyphs()).into_any_element()];
                        let width = (inner - (columns - 1) as f32 * COLUMN_GAP) / columns as f32;
                        shortcut(ui, (*title).to_string().into(), hit.label.clone(), cap)
                            .w(px(width.max(0.0)))
                    },
                )))
        });
        let empty = found.shown == 0;
        let footer = if !query.trim().is_empty() {
            div().child(format!(
                "{} of {} · matches labels and keys (“{}” finds {})",
                found.shown,
                found.total,
                keymap::glyphs(ActionId::CloseTerminal),
                "Close terminal"
            ))
        } else {
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_1()
                .child(style::icon(IconName::Globe))
                .map(|d| match &self.hotkey {
                    Some(hotkey) => d
                        .child("Show or hide Wormhole from any app")
                        .child(style::keycap(ui, hotkey.clone()))
                        .child(div().text_color(ui.faint).child("set in Settings › Menu bar & hotkey")),
                    None => d.child("A key that shows or hides Wormhole from any app is set in Settings › Menu bar & hotkey"),
                })
                .child(div().flex_1())
                .child("Also in Help › Keyboard Shortcuts and the palette")
        };
        let card = style::palette_surface(ui, true)
            .id("shortcut-sheet")
            .w(px(width))
            .max_h(px(max_h))
            .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                let _ = this.workspace.update(cx, |ws, cx| ws.close_overlay(window, cx));
            }))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_3()
                    .h(px(HEADER_H))
                    .pl(px(MARGIN))
                    .pr_4()
                    .rule_bottom(ui)
                    .child(div().text_color(ui.muted).child(style::icon(IconName::Keyboard)))
                    .child(div().text_heading().font_weight(FontWeight::BOLD).child("Keyboard shortcuts"))
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap_2()
                            .w(px(FILTER_W))
                            .h(px(style::CONTROL_H))
                            .px_2()
                            .rounded(ui.radius)
                            .outlined(ui, ui.border_strong)
                            .bg(ui.input)
                            .child(div().text_color(ui.muted).child(style::icon(IconName::Search)))
                            .child(
                                div().flex_1().min_w_0().child(Input::new(&self.filter).appearance(false)),
                            ),
                    )
                    .child(style::keycap(ui, "esc")),
            )
            .child(
                div()
                    .id("shortcut-sheet-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .when(empty, |d| {
                        d.child(
                            div()
                                .px(px(MARGIN))
                                .py_4()
                                .text_body()
                                .text_color(ui.muted)
                                .child("No shortcut matches"),
                        )
                    })
                    .when(!empty, |d| d.child(body))
                    .children(project_block),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .h(px(FOOTER_H))
                    .px(px(MARGIN))
                    .rule_top(ui)
                    .text_caption()
                    .text_color(ui.muted)
                    .child(footer),
            );
        scrim("shortcut-sheet-scrim", cx)
            .child(div().absolute().top(px(TOP)).left_0().right_0().flex().justify_center().child(card))
    }
}

#[cfg(feature = "devctl")]
impl Workspace {
    /// devctl `sheet [filter]` / `sheet close`: open the sheet over this window with `filter` typed;
    /// to an open one, type it in place of what is there.
    pub(crate) fn devctl_sheet(&mut self, arg: &str, window: &mut Window, cx: &mut Context<Self>) {
        let arg = arg.trim().to_string();
        if arg == "close" {
            self.close_overlay(window, cx);
            return;
        }
        if let Some(sheet) = self.sheet() {
            sheet.update(cx, |sheet, cx| {
                sheet.filter.update(cx, |filter, cx| filter.set_value(arg, window, cx));
                cx.notify();
            });
            return;
        }
        let Some(own) = app(cx).windows.iter().find(|w| w.id == self.window_id).cloned() else { return };
        cx.defer(move |cx| open_in(&own, &arg, cx));
    }
}
