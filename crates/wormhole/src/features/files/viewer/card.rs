//! Drawing the viewer: the card laid over the terminal, its header (the file's path, its size, Open in…,
//! Reveal, Copy Path and close) and the body for what the file is: lines in a virtual list with their
//! numbers, a picture scaled to fit, or a panel that says why there is nothing to read.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::platform::open_file;
use wormhole_core::platform::open_in;
use wormhole_core::storage::files::Problem;
use wormhole_core::theme::fonts::FontSlot;
use wormhole_text::{keys, text};

use super::{Content, TextShown, Viewer};
use crate::features::files::{Files, open};
use crate::fonts::font_in;
use crate::open_in::OpenIn;
use crate::settings::{UiColors, settings_in, store_in, ui_in};
use crate::style::{self, Edges as _, TextRole as _, panel_list};
use crate::theme_bridge::token;
use crate::window::facts::WindowFacts;

/// The card's header is a title row tall; the buttons are the small ones of the diff's narrow header.
const BUTTON_H: f32 = 24.0;
/// Roughly the advance of a monospace glyph, in ems.
const CHAR_EM: f32 = 0.6;
/// Space around a line number in the gutter (left, right).
const GUTTER_PAD: f32 = 12.0 + 8.0;
/// The room after the code in its column.
const CODE_END_PAD: f32 = 24.0;

/// What the lines are drawn with, fixed for a frame.
#[derive(Clone)]
struct LineStyle {
    family: SharedString,
    size: f32,
    height: f32,
    gutter: f32,
    scroll_x: f32,
    number: Hsla,
    text: Hsla,
}

impl Files {
    /// The viewer's card, inset in the column the terminal is in (`None`: nothing is open).
    pub(in crate::features::files) fn render_viewer(
        &mut self,
        facts: &WindowFacts<'_>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.follow(facts);
        let terminal = facts.active()?.terminal.clone();
        let viewer = self.viewer.as_ref()?;
        let ui = ui_in(&self.profile, cx);
        let surface = token(&store_in(&self.profile, cx).tokens, "surface.window");
        let header = self.render_viewer_header(viewer, ui, &terminal, cx);
        let body = self.render_viewer_body(viewer, ui, cx);
        let card = div()
            .id("file-viewer")
            .debug_selector(|| "file-viewer".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .rounded(ui.radius_lg)
            .outlined(ui, ui.border)
            .bg(surface)
            .aria_label(SharedString::from(keys::files_spoken_viewer(&viewer.name())))
            .child(header)
            .child(body);
        Some(
            div().id("file-viewer-column").size_full().flex().flex_col().p_4().child(card).into_any_element(),
        )
    }

    fn render_viewer_header(
        &self,
        viewer: &Viewer,
        ui: UiColors,
        terminal: &Entity<crate::terminal_view::TerminalView>,
        cx: &mut Context<Self>,
    ) -> Div {
        let (family, _) = font_in(&self.profile, FontSlot::Terminal, cx);
        let name = viewer.name();
        let dir = match viewer.path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => format!("{}/", dir.display()),
            _ => String::new(),
        };
        let units = settings_in(&self.profile, cx).size_units;
        let size = match &viewer.content {
            Content::Text(shown) => Some(shown.size),
            Content::Binary { size } | Content::Image { size, .. } => Some(*size),
            _ => None,
        };
        let resolved = OpenIn::resolved(cx).app.bundle;
        let bundle = open::default_bundle(&resolved).to_string();
        let app = open_in::known(&bundle).map_or("", |editor| editor.name);
        let tooltip = open_file::tooltip(&bundle, app, None);
        let terminal = terminal.clone();
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(style::TITLE_ROW_H))
            .gap_2()
            .pl_4()
            .pr_1()
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
                            .overflow_hidden()
                            .items_center()
                            .font_family(family)
                            .text_body()
                            // The folder gives way first; the file's name stays whole as long as it can.
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
                                    .flex_shrink(0.01)
                                    .truncate()
                                    .text_color(ui.text)
                                    .child(wormhole_core::view::file_tree::names::clean(&name)),
                            ),
                    )
                    .children(size.map(|size| {
                        div()
                            .flex_none()
                            .text_caption()
                            .text_color(ui.muted)
                            .child(wormhole_core::git::byte_size(size, units))
                    })),
            )
            .child(
                style::icon_button("file-open-in", ui, IconName::ExternalLink, tooltip)
                    .in_title_row()
                    .on_click(cx.listener(|this, _, _, cx| this.viewer_open(cx))),
            )
            .child(
                style::icon_button("file-reveal", ui, IconName::FolderOpen, text(keys::FILES_ROW_REVEAL))
                    .in_title_row()
                    .on_click(cx.listener(|this, _, _, cx| this.viewer_reveal(cx))),
            )
            .child(
                style::icon_button("file-copy-path", ui, IconName::Copy, text(keys::FILES_ROW_COPY_PATH))
                    .in_title_row()
                    .on_click(cx.listener(|this, _, window, cx| this.viewer_copy_path(window, cx))),
            )
            .child(
                style::icon_button("file-close", ui, IconName::X, text(keys::FILES_VIEWER_CLOSE))
                    .in_title_row()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.close_viewer(&terminal, window, cx);
                    })),
            )
    }

    fn render_viewer_body(&self, viewer: &Viewer, ui: UiColors, cx: &mut Context<Self>) -> Div {
        let body = div().relative().flex_1().min_h_0().flex().flex_col();
        match &viewer.content {
            Content::Reading => body.child(style::empty_note(ui, text(keys::FILES_READING))),
            Content::Text(shown) => self.render_text(viewer, shown, ui, body, cx),
            Content::Binary { size } => {
                let units = settings_in(&self.profile, cx).size_units;
                let words = keys::files_viewer_binary(&wormhole_core::git::byte_size(*size, units));
                body.child(self.render_panel(ui, words, true, cx))
            }
            Content::Image { image: Some(image), .. } => {
                body.child(
                    div().flex_1().min_h_0().p_4().child(
                        img(image.clone()).id("file-image").size_full().object_fit(ObjectFit::ScaleDown),
                    ),
                )
            }
            Content::Image { image: None, .. } => {
                body.child(self.render_panel(ui, text(keys::FILES_VIEWER_IMAGE_FAILED).to_string(), true, cx))
            }
            Content::Unavailable(problem) => {
                let words = match problem {
                    Problem::Gone => keys::FILES_VIEWER_GONE,
                    Problem::Outside => keys::FILES_VIEWER_OUTSIDE,
                    Problem::NotAFile => keys::FILES_VIEWER_NOT_FILE,
                    Problem::Denied => keys::FILES_VIEWER_DENIED,
                    Problem::Other => keys::FILES_VIEWER_UNREADABLE,
                };
                // A deleted file has nothing to open or show; the others might still be revealed.
                let actions = !matches!(problem, Problem::Gone | Problem::Outside);
                body.child(self.render_panel(ui, text(words).to_string(), actions, cx))
            }
        }
    }

    /// A note in the middle of the card, with Open in… and Reveal under it when they have something to act on.
    fn render_panel(&self, ui: UiColors, words: String, actions: bool, cx: &mut Context<Self>) -> Div {
        let button = |id: &'static str, label: &'static str| {
            Button::new(id).outline().xsmall().h(px(BUTTON_H)).px_2().label(label)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .p_4()
            .child(style::icon(IconName::File).text_color(ui.muted))
            .child(div().text_body().text_color(ui.text).text_center().child(words))
            .when(actions, |d| {
                d.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button("file-panel-open-in", text(keys::FILES_VIEWER_OPEN_IN))
                                .primary()
                                .on_click(cx.listener(|this, _, _, cx| this.viewer_open(cx))),
                        )
                        .child(
                            button("file-panel-reveal", text(keys::FILES_ROW_REVEAL))
                                .on_click(cx.listener(|this, _, _, cx| this.viewer_reveal(cx))),
                        ),
                )
            })
    }

    /// The text: numbered lines in a virtual list, scrolled sideways with the wheel while the numbers stay.
    fn render_text(
        &self,
        viewer: &Viewer,
        shown: &TextShown,
        ui: UiColors,
        body: Div,
        cx: &mut Context<Self>,
    ) -> Div {
        if shown.lines.len() == 0 {
            return body.child(style::empty_note(ui, text(keys::FILES_VIEWER_EMPTY)));
        }
        let store = store_in(&self.profile, cx);
        let (family, size) = font_in(&self.profile, FontSlot::Terminal, cx);
        let size = f32::from(size);
        let digits = shown.lines.len().to_string().len().max(3);
        let char_width = size * CHAR_EM;
        let gutter = (digits as f32 * char_width + GUTTER_PAD).ceil();
        // Sideways: as far as the longest line is wider than the room beside the numbers.
        let room = viewer.body_width.get() - gutter;
        let max_scroll = (shown.lines.widest() as f32 * char_width + CODE_END_PAD - room).max(0.0);
        viewer.max_scroll_x.set(max_scroll);
        let style = LineStyle {
            family,
            size,
            height: (size * 1.6).round().max(20.0),
            gutter,
            scroll_x: viewer.scroll_x.min(max_scroll),
            number: token(&store.tokens, "diff.line_number"),
            text: ui.text,
        };
        let (lines, colors, cache) = (shown.lines.clone(), shown.colors.clone(), shown.rows.clone());
        let (theme, tab_width) = (store.highlight.clone(), settings_in(&self.profile, cx).tab_width);
        let style = Rc::new(style);
        let mut list = uniform_list("file-lines", lines.len(), move |range, _, _| {
            range
                .map(|ix| {
                    let row = cache.row(ix, &lines, colors.as_deref(), &theme, tab_width);
                    line(ix, &row, &style).into_any_element()
                })
                .collect()
        })
        .track_scroll(&viewer.scroll)
        .size_full();
        // A sideways swipe scrolls the code, never the list up or down.
        list.style().restrict_scroll_to_axis = Some(true);
        let measure = viewer.body_width.clone();
        let scrollbar =
            panel_list::scroll_edges(ui, false, Scrollbar::vertical(&viewer.scroll).id("file-scrollbar"), cx);
        body.child(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                    this.scroll_viewer_sideways(event, window, cx);
                }))
                .child(
                    canvas(move |bounds, _, _| measure.set(f32::from(bounds.size.width)), |_, _, _, _| {})
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                )
                .child(list)
                .child(scrollbar),
        )
        .when(shown.truncated, |d| {
            let units = settings_in(&self.profile, cx).size_units;
            let cut = wormhole_core::storage::files::TEXT_CAP;
            let words = keys::files_viewer_truncated(
                &wormhole_core::git::byte_size(cut, units),
                &wormhole_core::git::byte_size(shown.size, units),
            );
            d.child(
                div().flex_none().px_4().py_2().rule_top(ui).text_caption().text_color(ui.muted).child(words),
            )
        })
    }

    /// Sideways scrolling moves the code only, so the numbers stay in view.
    fn scroll_viewer_sideways(&mut self, event: &ScrollWheelEvent, window: &Window, cx: &mut Context<Self>) {
        let Some(viewer) = self.viewer.as_mut() else { return };
        let mut delta = event.delta.pixel_delta(window.line_height());
        if event.delta.precise() {
            viewer.gesture.filter(&mut delta, event.touch_phase);
        }
        let delta = f32::from(delta.x);
        if delta == 0.0 {
            return;
        }
        let next = (viewer.scroll_x - delta).clamp(0.0, viewer.max_scroll_x.get());
        if next != viewer.scroll_x {
            viewer.scroll_x = next;
            cx.notify();
        }
    }

    /// Open in…: the app a double-click opens a file in, or Finder showing it.
    pub(super) fn viewer_open(&mut self, cx: &mut Context<Self>) {
        let Some(viewer) = &self.viewer else { return };
        let resolved = OpenIn::resolved(cx).app.bundle;
        open::open_in(open::default_bundle(&resolved), &viewer.root, &viewer.as_row(), cx);
    }

    pub(super) fn viewer_reveal(&mut self, _: &mut Context<Self>) {
        if let Some(viewer) = &self.viewer {
            open::reveal(&viewer.root, &viewer.as_row());
        }
    }

    pub(super) fn viewer_copy_path(&mut self, window: &Window, cx: &mut Context<Self>) {
        if let Some(viewer) = &self.viewer {
            open::copy_path(&viewer.root, &viewer.as_row(), window, cx);
        }
    }
}

/// One numbered line.
fn line(ix: usize, row: &super::lines::Row, style: &LineStyle) -> Stateful<Div> {
    let code = StyledText::new(row.text.clone()).with_highlights(row.colors.iter().cloned());
    div()
        .id(("file-line", ix))
        .flex()
        .w_full()
        .h(px(style.height))
        .items_center()
        .whitespace_nowrap()
        .font_family(style.family.clone())
        .text_size(px(style.size))
        .child(
            div()
                .w(px(style.gutter))
                .flex_none()
                .pr_2()
                .text_right()
                .text_color(style.number)
                .child(row.number.to_string()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_color(style.text)
                .child(div().ml(px(-style.scroll_x)).child(code)),
        )
}
