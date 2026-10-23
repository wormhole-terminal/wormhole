//! The sheet's card and its steps: "Step N of 3" and Skip setup on top, the
//! title and a line under it, the step, and Back, the three dots and Continue (Start on the
//! last step) along the bottom. The first step lists the places a Finder does instead of
//! a scan of ~/Projects, and takes a folder dropped on the card.

use std::path::Path;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::onboarding::{Event, Place, Step};
use wormhole_core::onboarding::{PlaceKind, STEPS};
use wormhole_core::platform::paths;

use super::{FirstRun, Intro};
use crate::settings::{UiColors, ui};
use crate::style::{self, Edges, TextRole};

// The sheet's card size.
const SHEET_W: f32 = 432.0;
const SHEET_MIN_H: f32 = 640.0;
/// How far the sheet stays from the window's top and bottom.
const SHEET_MARGIN: f32 = 32.0;
const FOOTER_H: f32 = 56.0;
/// The step dots: the current one a bar, the others dots.
const DOT: f32 = 4.0;
const DOT_BAR: f32 = 16.0;
/// The dashed box a folder can be dropped on.
const DROP_ZONE_H: f32 = 40.0;

impl FirstRun {
    pub(super) fn render_sheet(
        &self,
        sheet: &Intro,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let step = sheet.model.step();
        let (title, intro): (&str, SharedString) = match step {
            Step::Folder => (
                "Where do you work?",
                "Wormhole is a terminal first. Pick a folder to start in; every terminal remembers its own."
                    .into(),
            ),
            Step::Look => (
                "Make it yours",
                format!(
                    "Everything changes live, and all of it is in Settings ({}) later.",
                    keymap::glyphs(ActionId::Settings)
                )
                .into(),
            ),
            Step::Agents => {
                let found = sheet.agents.as_ref().map_or(0, |a| a.found.len());
                (
                    "Coding agents",
                    format!(
                        "Found {found} on this Mac. Install status hooks for the ones you want; they report \
                         when an agent works, waits for you or is done."
                    )
                    .into(),
                )
            }
            Step::Primer => (
                "Ready when you are",
                "Wormhole didn't find a coding agent on this Mac. Nothing here depends on one.".into(),
            ),
            // The welcome page is the intro window's own (`welcome_page.rs`), not a card.
            Step::Welcome => ("Welcome to Wormhole", SharedString::default()),
        };
        let counter = match step {
            Step::Primer => format!("Step {} of {STEPS} · no agents found", step.number()),
            _ => format!("Step {} of {STEPS}", step.number()),
        };
        let body = match step {
            Step::Folder => self.render_folder_step(sheet, ui, cx),
            Step::Look => self.render_look_step(ui, cx),
            Step::Agents => self.render_agents_step(sheet, ui, cx),
            Step::Primer => self.render_primer_step(ui, cx),
            Step::Welcome => div().into_any_element(),
        };
        let max_h = window.viewport_size().height - px(2.0 * SHEET_MARGIN);
        div()
            .id("onboarding-sheet")
            .w(px(SHEET_W))
            .min_h(px(SHEET_MIN_H).min(max_h))
            .max_h(max_h)
            .flex()
            .flex_col()
            .rounded(ui.radius_lg)
            .overflow_hidden()
            .outlined(ui, ui.border_strong)
            .bg(ui.elevated)
            .shadow(select_shadow(ui))
            // Clicks on the card stay on it.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // A folder dropped anywhere on the first step counts, not only on its dashed box.
            .when(step == Step::Folder, |d| {
                d.on_drop(cx.listener(|this, files: &ExternalPaths, window, cx| {
                    this.drop_folder(files, window, cx);
                }))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_6()
                    .pt_6()
                    .pb_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_caption()
                            .text_color(ui.muted)
                            .child(counter)
                            .child(
                                Button::new("onboarding-skip").ghost().xsmall().label("Skip setup").on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.onboarding_event(Event::Skip, window, cx)
                                    }),
                                ),
                            ),
                    )
                    .child(div().pt_2().text_title().font_weight(FontWeight::BOLD).child(title))
                    .child(div().text_body().text_color(ui.muted).child(intro)),
            )
            // A column, so a step is as wide as the card and its text wraps there.
            .child(
                div()
                    .id("onboarding-body")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(self.render_footer(sheet, ui, cx))
            .into_any_element()
    }

    /// Back, where the sheet is (three dots), and Continue or Start.
    fn render_footer(&self, sheet: &Intro, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let step = sheet.model.step();
        let dots = div().flex().items_center().gap_1().children((1..=STEPS).map(|n| {
            let current = n == step.number();
            div().h(px(DOT)).w(px(if current { DOT_BAR } else { DOT })).rounded_full().bg(if current {
                ui.primary
            } else {
                ui.faint
            })
        }));
        let (label, event) =
            if step.is_last() { ("Start", Event::Start) } else { ("Continue", Event::Continue) };
        div()
            .flex_none()
            .flex()
            .items_center()
            .h(px(FOOTER_H))
            .px_6()
            .rule_top(ui)
            .child(
                div().flex_1().child(
                    Button::new("onboarding-back")
                        .ghost()
                        .small()
                        .icon(IconName::ChevronLeft)
                        .label("Back")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.onboarding_event(Event::Back, window, cx)),
                        ),
                ),
            )
            .child(dots)
            .child(div().flex_1().flex().justify_end().child(
                Button::new("onboarding-next").primary().small().label(label).on_click(
                    cx.listener(move |this, _, window, cx| this.onboarding_event(event.clone(), window, cx)),
                ),
            ))
            .into_any_element()
    }

    /// Step 1: open a folder (⌘O), or pick one of the places a Finder starts with: the folders
    /// worked in lately, Home, Desktop, Documents and Developer. The folder picked (or dropped
    /// on the card) takes the primary's tint; Continue goes on without one.
    fn render_folder_step(&self, sheet: &Intro, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let picked = sheet.model.folder().cloned();
        let picked_place =
            picked.as_ref().is_some_and(|folder| sheet.places.iter().any(|p| &p.path == folder));
        let open_on = picked.is_some() && !picked_place;
        let open_detail: SharedString = match picked.as_ref().filter(|_| open_on) {
            Some(folder) => format!("Starts in {}", short_home(folder)).into(),
            None => "A terminal starts there; git changes, history and branch follow it.".into(),
        };
        let open = style::flush_row("onboarding-open-folder", ui, open_on)
            .py_2()
            .items_start()
            .child(style::lead(style::icon(IconName::Folder).text_color(if open_on {
                ui.selected_text
            } else {
                ui.muted
            })))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().text_body().font_weight(FontWeight::MEDIUM).child("Open folder…"))
                    .child(
                        div()
                            .text_caption()
                            .text_color(if open_on { ui.selected_muted } else { ui.muted })
                            .child(open_detail),
                    ),
            )
            .child(style::keycap(ui, keymap::glyphs(ActionId::OpenFolder)))
            .on_click(cx.listener(|this, _, _, cx| this.pick_folder(cx)));
        let place_row = |place: &Place| {
            let on = picked.as_deref() == Some(place.path.as_path());
            let path = place.path.clone();
            style::flush_row(SharedString::from(format!("onboarding-place-{}", place.path.display())), ui, on)
                .py_1()
                .child(style::lead(style::icon(place_icon(place.kind)).text_color(if on {
                    ui.selected_text
                } else {
                    ui.muted
                })))
                .child(div().child(place.name.clone()))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_caption()
                        .text_color(if on { ui.selected_muted } else { ui.faint })
                        .child(place.detail.clone()),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.onboarding_event(Event::PickFolder(path.clone()), window, cx)
                }))
        };
        let (recents, standard): (Vec<&Place>, Vec<&Place>) =
            sheet.places.iter().partition(|place| place.kind == PlaceKind::Recent);
        let zone = div()
            .id("onboarding-drop-zone")
            .mx_4()
            .mt_4()
            .h(px(DROP_ZONE_H))
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .rounded(ui.radius)
            .outlined(ui, ui.border_strong)
            .border_dashed()
            .text_caption()
            .text_color(ui.muted)
            .drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(ui.highlight))
            .on_drop(cx.listener(|this, files: &ExternalPaths, window, cx| {
                // The card underneath takes drops too; once is enough.
                cx.stop_propagation();
                this.drop_folder(files, window, cx);
            }))
            .child(style::icon(IconName::FolderInput))
            .child("Or drop a folder here");
        div()
            .flex()
            .flex_col()
            .rule_top(ui)
            .child(open)
            .when(!recents.is_empty(), |d| {
                d.child(place_label(ui, "Recents")).children(recents.iter().map(|place| place_row(place)))
            })
            .child(place_label(ui, "Places"))
            .children(standard.iter().map(|place| place_row(place)))
            .child(zone)
            .into_any_element()
    }
}

/// The caption over a group of places ("PLACES"): at the bottom of a row, as the settings
/// nav's group labels.
#[track_caller]
fn place_label(ui: UiColors, text: &str) -> Div {
    div().flex().items_end().h(px(style::ROW_H)).px_4().pb_1().child(style::section_label(ui, text))
}

fn place_icon(kind: PlaceKind) -> IconName {
    match kind {
        PlaceKind::Recent => IconName::Clock,
        PlaceKind::Home => IconName::House,
        PlaceKind::Desktop => IconName::Monitor,
        PlaceKind::Documents => IconName::FileText,
        PlaceKind::Developer => IconName::Hammer,
    }
}

/// The card's shadow: it floats over the window.
fn select_shadow(ui: UiColors) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: ui.shadow,
        offset: point(px(0.0), px(SHADOW_Y)),
        blur_radius: px(SHADOW_BLUR),
        spread_radius: px(0.0),
        inset: false,
    }]
}

const SHADOW_Y: f32 = 16.0;
const SHADOW_BLUR: f32 = 48.0;

/// `~/…` for a folder in the home folder.
fn short_home(path: &Path) -> String {
    let home = paths::home().unwrap_or_default();
    wormhole_core::onboarding::tilde(path, &home)
}
