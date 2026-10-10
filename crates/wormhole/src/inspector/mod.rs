//! Inspect mode (⇧⌥⌘I, debug builds or the `inspector` feature). Point at
//! anything in a window: it lights up, and a panel on the right says where it is
//! built (file:line and that line of code), what it is called (its element
//! path) and how it is styled, in the design language's terms (`_2`, `radius`,
//! `body`, token names). Padding, gap, corner radius, height and text size can
//! be nudged live. "Copy" puts a note with the location and every change on the
//! clipboard, to paste into a request.
//!
//! GPUI does the picking (hover highlights, click pins, the scroll wheel walks
//! out to the parents) and records where each element was made. GPUI Kit keeps
//! empty, window-sized layers (its root, sheets, dialogs, notifications) above
//! the app, and picking always starts at the top, so a pick that lands on one is
//! sent a step deeper, as if scrolled, until it reaches the app. The `style`
//! helpers are `#[track_caller]`, so a shared row reports the view that asked
//! for it rather than `style/`.
//!
//! `facts` describes a style in the design language's terms, `edit` is the style
//! section with its steppers, `sources` finds where an element was built.

mod edit;
mod facts;
#[cfg(feature = "devctl")]
pub(crate) mod probe;
mod sources;

use std::cell::RefCell;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme as _, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use wormhole_core::keymap::{self, ActionId};

use crate::settings::{SettingsStore, ui};
use crate::style::{self, Edges as _, TextRole};
use crate::theme_bridge::token;

actions!(developer, [Inspect]);

/// How far the panel starts below the window's top: clear of its title bar (38 pt) and the
/// traffic lights, which the panel's window spans under.
const PANEL_TOP: f32 = 40.0;

thread_local! {
    /// The note for the element in the panel, written while its style section
    /// renders so the header's Copy button can use it.
    static NOTE: RefCell<Option<(InspectorElementId, String)>> = const { RefCell::new(None) };
    /// Whether the window is picking, for the style section's render.
    static PICKING: RefCell<bool> = const { RefCell::new(false) };
    /// Layers skipped since the pick last landed on the app, and how many it took
    /// then: the layers don't change, so the next skip jumps past all of them at once.
    static SKIPPED: RefCell<(usize, usize)> = const { RefCell::new((0, 1)) };
}

pub fn init(cx: &mut App) {
    // GPUI Kit has an inspector of its own (a raw style editor) on ⌥⌘I, which is the right
    // panel's key now: `workspace::actions` unbinds it and binds the keymap's ⇧⌥⌘I to
    // `Inspect`. Setting the renderer after `gpui_kit::init` swaps in this panel.
    cx.on_action(|_: &Inspect, cx| {
        // The window is in the middle of dispatching the menu action; toggle once it's done.
        if let Some(handle) = cx.active_window() {
            cx.defer(move |cx| {
                handle.update(cx, |_, window, cx| window.toggle_inspector(cx)).ok();
            });
        }
    });
    cx.set_inspector_renderer(Box::new(render_panel));
    cx.register_inspector_element(|_, _| {
        // What the element looked like when it was picked, to list the changes.
        let mut original: Option<(InspectorElementId, StyleRefinement)> = None;
        move |id, state: &DivInspectorState, window: &mut Window, cx: &mut App| {
            edit::render_div(id, state, &mut original, window, cx)
        }
    });
}

fn render_panel(inspector: &mut Inspector, window: &mut Window, cx: &mut Context<Inspector>) -> AnyElement {
    #[cfg(feature = "devctl")]
    probe::drawn();
    facts::sync_token_colors(cx);
    let ui = ui(cx);
    let tokens = &cx.global::<SettingsStore>().tokens;
    let background = Hsla { a: 1.0, ..token(tokens, "surface.panel") };
    let (font, size) = (cx.theme().font_family.clone(), cx.theme().font_size);
    let picking = inspector.is_picking();
    PICKING.with(|p| *p.borrow_mut() = picking);
    let states = inspector.render_inspector_states(window, cx);
    let active = inspector.active_element_id().cloned();
    let this = cx.entity();

    let hint = if picking {
        "Hover to highlight, click to pin, scroll for the parent."
    } else {
        "Pinned. Nudge values below, then copy the note."
    };
    let header = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().text_heading().child("Inspector"))
                .when(!picking, |d| {
                    d.child(
                        Button::new("inspector-pick")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Inspector)
                            .label("Pick")
                            .on_click(move |_, window, cx| {
                                this.update(cx, |inspector, _| inspector.start_picking());
                                window.refresh();
                            }),
                    )
                })
                .when_some(active.clone(), |d, id| {
                    d.child(
                        Button::new("inspector-copy")
                            .primary()
                            .xsmall()
                            .icon(IconName::Copy)
                            .label("Copy")
                            .tooltip("Copy a note with the location and your changes")
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(note_for(&id)));
                            }),
                    )
                })
                .child(
                    style::icon_button(
                        "inspector-close",
                        ui,
                        IconName::X,
                        keymap::with_key("Close", ActionId::Inspector),
                    )
                    .on_click(|_, window: &mut Window, cx: &mut App| window.toggle_inspector(cx)),
                ),
        )
        .child(div().text_caption().text_color(ui.muted).child(hint));

    let body = match &active {
        None => style::empty_note(ui, "Point at anything in the window.").px_0().into_any_element(),
        Some(id) => {
            let location = id.path.source_location;
            let code = sources::source_line(location.file(), location.line());
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::section_label(ui, "Element"))
                .child(div().text_body().child(sources::element_name(id)))
                .child(div().text_caption().text_color(ui.primary).child(format!(
                    "{}:{}",
                    sources::short_file(location.file()),
                    location.line()
                )))
                .children(code.map(|code| {
                    div()
                        .px_2()
                        .py_1()
                        .rounded(ui.radius)
                        .bg(ui.hover)
                        .text_caption()
                        .font_family(crate::fonts::code_font(cx).0)
                        .child(code)
                }))
                .into_any_element()
        }
    };

    div()
        .id("inspector-panel")
        .size_full()
        .flex()
        .flex_col()
        .gap_4()
        .p_3()
        .pt(px(PANEL_TOP))
        .overflow_y_scroll()
        .bg(background)
        .rule_left(ui)
        .font_family(font)
        .text_size(size)
        .text_color(ui.text)
        .child(header)
        .child(body)
        .children(states)
        .into_any_element()
}

/// One of GPUI Kit's empty, window-sized layers (sheets, dialogs, notifications,
/// its root), never what anyone means to point at: a dependency's element with
/// no fill that spans the window from its top-left corner.
fn is_kit_layer(id: &InspectorElementId, state: &DivInspectorState, window: &Window) -> bool {
    let bounds = state.bounds;
    !id.path.source_location.file().starts_with("crates/")
        && state.base_style.background.is_none()
        && bounds.origin == point(px(0.0), px(0.0))
        && bounds.size.height >= window.viewport_size().height
}

/// Move the pick `steps` layers deeper, the way the scroll wheel does.
fn skip_layers(window: &mut Window, steps: usize) {
    let position = window.mouse_position();
    window.on_next_frame(move |window, cx| {
        let event = ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(0.0), px(36.0 * steps as f32))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        };
        window.dispatch_event(PlatformInput::ScrollWheel(event), cx);
    });
}

/// The note for `id`: the one its style section wrote, else just where it is.
fn note_for(id: &InspectorElementId) -> String {
    NOTE.with(|n| n.borrow().as_ref().filter(|(seen, _)| seen == id).map(|(_, note)| note.clone()))
        .unwrap_or_else(|| {
            let location = id.path.source_location;
            format!(
                "{}:{}  {}",
                sources::short_file(location.file()),
                location.line(),
                sources::element_name(id)
            )
        })
}

/// While picking, step past one of GPUI Kit's empty layers (and remember how many it
/// took), or note that the pick reached the app.
fn skip_kit_layers(id: &InspectorElementId, state: &DivInspectorState, window: &mut Window) {
    if PICKING.with(|p| *p.borrow()) {
        if is_kit_layer(id, state, window) {
            let steps = SKIPPED.with(|s| {
                let (run, learned) = &mut *s.borrow_mut();
                let steps = if *run == 0 { *learned } else { 1 };
                *run += steps;
                steps
            });
            skip_layers(window, steps);
        } else {
            SKIPPED.with(|s| {
                let (run, learned) = &mut *s.borrow_mut();
                if *run > 0 {
                    *learned = *run;
                    *run = 0;
                }
            });
        }
    }
}
