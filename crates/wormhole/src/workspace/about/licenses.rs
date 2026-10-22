//! The Licenses window: every crate Wormhole is built from on the left (searchable), the
//! selected one's license on the right. The data is `wormhole_core::licenses`.

use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::licenses::Licenses;

use super::super::panel_list::Rows;
use super::ui;
use crate::settings::UiColors;
use crate::style::{self, Edges, TextRole};
#[cfg(feature = "devctl")]
use crate::workspace::app::app;
use crate::workspace::aux_window;

const SIZE: (f32, f32) = (940.0, 660.0);
const MIN_SIZE: (f32, f32) = (640.0, 420.0);
const LIST_WIDTH: f32 = 300.0;

pub(crate) struct LicensesWindow {
    licenses: &'static Licenses,
    search: Entity<InputState>,
    /// The listed crates: indices into `licenses.crates()`, in list order.
    matches: Vec<usize>,
    list: Rows<usize>,
    selected: Option<usize>,
    text_scroll: ScrollHandle,
    focus_handle: FocusHandle,
    _search_changed: Subscription,
}

/// Open the Licenses window, or bring it to the front.
pub(crate) fn open_licenses(cx: &mut App) {
    let options = aux_window::window_options(
        "Licenses",
        size(px(SIZE.0), px(SIZE.1)),
        size(px(MIN_SIZE.0), px(MIN_SIZE.1)),
        cx,
    );
    aux_window::open_single(
        |state| &mut state.licenses_window,
        options,
        |window, cx| cx.new(|cx| LicensesWindow::new(window, cx)),
        cx,
    );
}

impl LicensesWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let licenses = Licenses::bundled();
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search packages or licenses"));
        let search_changed = cx.subscribe_in(&search, window, |this, input, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                let query = input.read(cx).value().to_string();
                this.filter(&query, cx);
            }
        });
        let matches: Vec<usize> = (0..licenses.crates().len()).collect();
        let mut list = Rows::new();
        list.set(matches.clone());
        search.update(cx, |search, cx| search.focus(window, cx));
        Self {
            licenses,
            search,
            selected: matches.first().copied(),
            matches,
            list,
            text_scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _search_changed: search_changed,
        }
    }

    fn filter(&mut self, query: &str, cx: &mut Context<Self>) {
        self.matches = self.licenses.search(query);
        self.list.set(self.matches.clone());
        // Keep the open license while it's still listed; otherwise show the first match.
        if !self.selected.is_some_and(|ix| self.matches.contains(&ix)) {
            self.select(self.matches.first().copied(), cx);
        }
        cx.notify();
    }

    fn select(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.selected = ix;
        self.text_scroll.set_offset(point(px(0.0), px(0.0)));
        cx.notify();
    }

    fn render_list(&self, ui: UiColors, cx: &mut Context<Self>) -> impl IntoElement {
        let (licenses, selected, window) = (self.licenses, self.selected, cx.weak_entity());
        self.list.element(None, move |&ix, _, _| {
            let krate = &licenses.crates()[ix];
            let window = window.clone();
            let is_selected = selected == Some(ix);
            let second = if is_selected { ui.selected_text } else { ui.muted };
            style::flush_row(("license-row", ix), ui, is_selected)
                .py_1()
                .on_click(move |_, _, cx| {
                    window.update(cx, |this, cx| this.select(Some(ix), cx)).ok();
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .gap_2()
                                .child(div().min_w_0().truncate().child(krate.name.clone()))
                                .child(
                                    div()
                                        .flex_none()
                                        .text_caption()
                                        .text_color(second)
                                        .child(krate.version.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_caption()
                                .text_color(second)
                                .truncate()
                                .child(krate.license.clone().unwrap_or_else(|| "no license declared".into())),
                        ),
                )
                .into_any_element()
        })
    }

    fn render_detail(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let Some(krate) = self.selected.map(|ix| &self.licenses.crates()[ix]) else {
            return style::empty_note(ui, "No package matches.").into_any_element();
        };
        let (family, size) = crate::fonts::code_font(cx);
        let texts: Vec<&str> = self.licenses.texts_of(krate).collect();

        let header = div()
            .flex()
            .flex_col()
            .gap_1()
            .pb_3()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(div().text_title().child(krate.name.clone()))
                    .child(div().text_body().text_color(ui.muted).child(krate.version.clone())),
            )
            .child(div().text_body().text_color(ui.muted).child(match &krate.license {
                Some(license) => format!("License: {license}"),
                None => "No license declared".to_string(),
            }))
            .children(krate.repository.clone().map(|url| {
                div()
                    .id("license-repository")
                    .text_body()
                    .text_color(ui.primary)
                    .cursor_pointer()
                    .hover(|d| d.underline())
                    .on_click({
                        let url = url.clone();
                        move |_, _, cx| cx.open_url(&url)
                    })
                    .child(url)
            }));

        let body = div()
            .id("license-text")
            .track_scroll(&self.text_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .pt_3()
            .rule_top(ui)
            .when(texts.is_empty(), |d| {
                d.child(div().text_body().text_color(ui.muted).child(
                    "This package doesn't include a license file. The license above is what its \
                     Cargo.toml declares; see the repository for the full text.",
                ))
            })
            .children(texts.into_iter().enumerate().map(|(ix, text)| {
                // The code font at its size: a license hard-wrapped at ~80 columns fits the pane.
                div()
                    .id(("license-file", ix))
                    .font_family(family.clone())
                    .text_size(size)
                    .child(text.to_string())
            }));

        div().flex().flex_col().size_full().child(header).child(body).into_any_element()
    }
}

impl Render for LicensesWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let surfaces = aux_window::surfaces(cx, false);
        let total = self.licenses.crates().len();
        let count = if self.matches.len() == total {
            format!("{total} packages")
        } else {
            format!("{} of {total} packages", self.matches.len())
        };

        // The list runs under the traffic lights, like the settings window's sidebar.
        let list_column = div()
            .flex()
            .flex_col()
            .w(px(LIST_WIDTH))
            .flex_none()
            .h_full()
            .pb_2()
            .pt(px(style::TITLE_ROW_H))
            .bg(surfaces.panel)
            .rule_right(ui)
            .child(div().px_4().pb_2().child(Input::new(&self.search).small()))
            .child(self.render_list(ui, cx))
            .child(div().pt_2().px_4().text_caption().text_color(ui.muted).child(count));

        let detail_column = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .px_6()
            .pb_4()
            .pt(px(style::TITLE_ROW_H))
            .bg(surfaces.background)
            .child(self.render_detail(ui, cx));

        div()
            .id("licenses-window")
            .track_focus(&self.focus_handle)
            .key_context("LicensesWindow")
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, _| {
                aux_window::close_on_esc_or_cmd_w(event, window);
            }))
            .size_full()
            .relative()
            .flex()
            .text_color(ui.text)
            // First, so it is behind what is drawn over it (`chrome::TitleDrag`).
            .child(aux_window::drag_strip())
            .child(list_column)
            .child(detail_column)
    }
}

/// devctl: `licenses-search <query>` fills the search box, `licenses-select <name>` opens the
/// first listed package whose name contains `name`.
#[cfg(feature = "devctl")]
pub(super) fn devctl(cmd: &str, arg: &str, cx: &mut App) -> anyhow::Result<String> {
    let (handle, view) =
        app(cx).licenses_window.clone().ok_or_else(|| anyhow::anyhow!("the window is not open"))?;
    let view = view
        .upgrade()
        .and_then(|view| view.downcast::<LicensesWindow>().ok())
        .ok_or_else(|| anyhow::anyhow!("the window is closed"))?;
    let arg = arg.trim().to_string();
    handle.update(cx, |_, window, cx| {
        view.update(cx, |this, cx| match cmd {
            "licenses-search" => {
                // Not `set_value`: that emits no Change event, which is what typing does.
                this.search.update(cx, |search, cx| search.replace_all(arg.clone(), window, cx));
                String::new()
            }
            _ => {
                let found =
                    this.matches.iter().copied().find(|&ix| this.licenses.crates()[ix].name.contains(&arg));
                this.select(found, cx);
                match found {
                    Some(ix) => format!("selected {}\n", this.licenses.crates()[ix].title()),
                    None => format!("no listed package matches {arg:?}\n"),
                }
            }
        })
    })
}
