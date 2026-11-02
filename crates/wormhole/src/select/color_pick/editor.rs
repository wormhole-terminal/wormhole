//! The editor under the presets: an area of saturation (across) and brightness (up), a hue slider, and
//! the hex field with the swatch of what they make.
//!
//! Both pointer controls take a press and a drag like the Kit's slider (a press moves the thumb where it
//! lands, a drag keeps moving it, past the edge it stops at the edge), and both are sliders for the
//! keyboard and for VoiceOver. The colours of the area and the bar are the colour model's own (white,
//! black and the hues round the wheel), not the theme's: they say what each place would pick.

use gpui_kit::accesskit::Role;
use gpui_kit::component::input::Input;
use gpui_kit::component::{ElementExt as _, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::{Color, Hsv};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{Pick, Picked, state::ColorPickState};
use crate::settings::UiColors;
use crate::style::{self, Edges as _, TextRole as _};
use crate::theme_bridge;

/// The area's height; its width is the card's.
const AREA_H: f32 = 120.0;
/// The hue bar's height, and the size of the area's thumb.
const BAR_H: f32 = 12.0;
const AREA_THUMB: f32 = 12.0;
/// The hue thumb is a little taller than the bar.
const BAR_THUMB_W: f32 = 8.0;
const BAR_THUMB_H: f32 = 16.0;
/// The swatch beside the hex field.
const SWATCH: f32 = 24.0;
/// A key moves the thumb by this much (a fraction of the area; degrees of hue), and with ⇧ by the bigger one.
const STEP: f32 = 0.01;
const STEP_BIG: f32 = 0.1;
const HUE_STEP: f32 = 1.0;
const HUE_STEP_BIG: f32 = 15.0;

#[derive(Clone)]
struct DragArea(EntityId);
#[derive(Clone)]
struct DragHue(EntityId);

impl Render for DragArea {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl Render for DragHue {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Where `at` falls in `bounds`, as fractions of its width and height, kept inside: a pointer beyond the
/// edge asks for the edge.
fn fractions(bounds: Bounds<Pixels>, at: Point<Pixels>) -> (f32, f32) {
    let across = |from: Pixels, size: Pixels| {
        if f32::from(size) <= 0.0 { 0.0 } else { (f32::from(from) / f32::from(size)).clamp(0.0, 1.0) }
    };
    (across(at.x - bounds.left(), bounds.size.width), across(at.y - bounds.top(), bounds.size.height))
}

/// Make `change(hsv)` the editor's colour and hand its hex to the setting.
fn move_to(
    state: &Entity<ColorPickState>,
    pick: &Pick,
    change: impl FnOnce(Hsv) -> Hsv,
    window: &mut Window,
    cx: &mut App,
) {
    let hex = state.update(cx, |state, cx| {
        let hsv = change(state.hsv);
        state.set(hsv, window, cx)
    });
    pick(Picked::Hex(hex), cx);
    window.refresh();
}

/// The editor: the area, the hue bar, and the hex row with `done` closing the card.
pub(super) fn editor(
    ui: UiColors,
    state: &Entity<ColorPickState>,
    pick: &Pick,
    done: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    let (hsv, bad, swatch, hex_field, bounds) = {
        let s = state.read(cx);
        (s.hsv, s.bad_hex, s.shown(), s.hex.clone(), (s.area.clone(), s.hue.clone()))
    };
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(area(ui, state, pick, hsv, bounds.0, cx))
        .child(hue_bar(ui, state, pick, hsv, bounds.1, cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(style::color_dot(ui, swatch, SWATCH))
                .child(
                    div()
                        .id("color-pick-hex")
                        .flex_1()
                        .min_w_0()
                        .role(Role::TextInput)
                        .aria_label(text(COLOR_PICK_HEX))
                        .child(Input::new(&hex_field).small().appearance(false)),
                )
                .child(
                    style::form_button("color-pick-done")
                        .primary()
                        .label(text(COLOR_PICK_DONE))
                        .on_click(move |_, window, cx| done(window, cx)),
                ),
        )
        .when(bad, |d| {
            d.child(div().text_caption().text_color(ui.danger_text).child(text(COLOR_PICK_HEX_BAD)))
        })
}

/// The model's white and black, which the area fades the hue into.
fn model(color: Color) -> Hsla {
    theme_bridge::hsla(color)
}

/// Saturation across, brightness up: the hue's own colour, faded to white towards the left and to black
/// towards the bottom.
fn area(
    ui: UiColors,
    state: &Entity<ColorPickState>,
    pick: &Pick,
    hsv: Hsv,
    place: super::state::Place,
    cx: &App,
) -> Stateful<Div> {
    let id = state.entity_id();
    let focus = state.read(cx).area_focus.clone();
    let (white, black) = (model(Color::WHITE), model(Color::BLACK));
    let clear = |color: Hsla| Hsla { a: 0.0, ..color };
    let (down, drag, key) = (state.clone(), state.clone(), state.clone());
    let (down_pick, drag_pick, key_pick) = (pick.clone(), pick.clone(), pick.clone());
    let thumb_ring = UiColors { border_width: px(2.0), ..ui };
    div()
        .id("color-pick-area")
        .role(Role::Slider)
        .aria_label(text(COLOR_PICK_AREA))
        .aria_value(color_pick_area_value(&percent(hsv.s).to_string(), &percent(hsv.v).to_string()))
        .track_focus(&focus)
        .focus_visible(move |d| d.ringed(ui, ui.focus, ui.focus_ring_width))
        .relative()
        .w_full()
        .h(px(AREA_H))
        .rounded(ui.radius_sm)
        .overflow_hidden()
        .cursor_crosshair()
        .bg(model(hsv.pure_hue()))
        .child(div().absolute().size_full().bg(linear_gradient(
            90.0,
            linear_color_stop(white, 0.0),
            linear_color_stop(clear(white), 1.0),
        )))
        .child(div().absolute().size_full().bg(linear_gradient(
            0.0,
            linear_color_stop(black, 0.0),
            linear_color_stop(clear(black), 1.0),
        )))
        .child(
            div()
                .absolute()
                .left(relative(hsv.s))
                .top(relative(1.0 - hsv.v))
                .ml(px(-AREA_THUMB / 2.0))
                .mt(px(-AREA_THUMB / 2.0))
                .size(px(AREA_THUMB))
                .rounded_full()
                .outlined_always(thumb_ring, model(Color::WHITE))
                .shadow(vec![ring_shadow()]),
        )
        .on_prepaint(move |bounds, _, _| place.set(bounds))
        .on_mouse_down(MouseButton::Left, {
            let (place, focus) = (state.read(cx).area.clone(), focus.clone());
            move |event, window, cx| {
                window.focus(&focus, cx);
                let (x, y) = fractions(place.get(), event.position);
                move_to(&down, &down_pick, |hsv| Hsv::new(hsv.h, x, 1.0 - y), window, cx);
            }
        })
        .on_drag(DragArea(id), |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
        })
        .on_drag_move(move |event: &DragMoveEvent<DragArea>, window, cx| {
            if event.drag(cx).0 == id {
                let (x, y) = fractions(event.bounds, event.event.position);
                move_to(&drag, &drag_pick, |hsv| Hsv::new(hsv.h, x, 1.0 - y), window, cx);
            }
        })
        .on_key_down(move |event, window, cx| {
            let step = if event.keystroke.modifiers.shift { STEP_BIG } else { STEP };
            let (ds, dv) = match event.keystroke.key.as_str() {
                "left" => (-step, 0.0),
                "right" => (step, 0.0),
                "up" => (0.0, step),
                "down" => (0.0, -step),
                _ => return,
            };
            cx.stop_propagation();
            move_to(&key, &key_pick, |hsv| Hsv::new(hsv.h, hsv.s + ds, hsv.v + dv), window, cx);
        })
}

/// The hue, round the wheel from red to red, as six runs of a two-colour gradient (GPUI's gradients have
/// two stops).
fn hue_bar(
    ui: UiColors,
    state: &Entity<ColorPickState>,
    pick: &Pick,
    hsv: Hsv,
    place: super::state::Place,
    cx: &App,
) -> Stateful<Div> {
    let id = state.entity_id();
    let focus = state.read(cx).hue_focus.clone();
    let (down, drag, key) = (state.clone(), state.clone(), state.clone());
    let (down_pick, drag_pick, key_pick) = (pick.clone(), pick.clone(), pick.clone());
    let at = |degrees: f32| model(Hsv::new(degrees, 1.0, 1.0).to_color());
    let runs = (0..6).map(|run| {
        let from = run as f32 * 60.0;
        div().h_full().flex_1().bg(linear_gradient(
            90.0,
            linear_color_stop(at(from), 0.0),
            linear_color_stop(at(from + 60.0), 1.0),
        ))
    });
    div()
        .id("color-pick-hue")
        .role(Role::Slider)
        .aria_label(text(COLOR_PICK_HUE))
        .aria_value(color_pick_hue_value(&(hsv.h.round() as usize).to_string()))
        .track_focus(&focus)
        .focus_visible(move |d| d.ringed(ui, ui.focus, ui.focus_ring_width))
        .relative()
        .w_full()
        .h(px(BAR_H))
        .rounded(ui.radius_sm)
        .cursor_pointer()
        .child(div().flex().size_full().rounded(ui.radius_sm).overflow_hidden().children(runs))
        .child(
            div()
                .absolute()
                .left(relative(hsv.h / 360.0))
                .top(px((BAR_H - BAR_THUMB_H) / 2.0))
                .ml(px(-BAR_THUMB_W / 2.0))
                .w(px(BAR_THUMB_W))
                .h(px(BAR_THUMB_H))
                .rounded(ui.radius_sm)
                .bg(model(Color::WHITE))
                .shadow(vec![ring_shadow()]),
        )
        .on_prepaint(move |bounds, _, _| place.set(bounds))
        .on_mouse_down(MouseButton::Left, {
            let (place, focus) = (state.read(cx).hue.clone(), focus.clone());
            move |event, window, cx| {
                window.focus(&focus, cx);
                let (x, _) = fractions(place.get(), event.position);
                move_to(&down, &down_pick, |hsv| Hsv::new(x * 360.0, hsv.s, hsv.v), window, cx);
            }
        })
        .on_drag(DragHue(id), |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
        })
        .on_drag_move(move |event: &DragMoveEvent<DragHue>, window, cx| {
            if event.drag(cx).0 == id {
                let (x, _) = fractions(event.bounds, event.event.position);
                move_to(&drag, &drag_pick, |hsv| Hsv::new(x * 360.0, hsv.s, hsv.v), window, cx);
            }
        })
        .on_key_down(move |event, window, cx| {
            let step = if event.keystroke.modifiers.shift { HUE_STEP_BIG } else { HUE_STEP };
            let by = match event.keystroke.key.as_str() {
                "left" | "down" => -step,
                "right" | "up" => step,
                _ => return,
            };
            cx.stop_propagation();
            move_to(&key, &key_pick, |hsv| Hsv::new(hsv.h + by, hsv.s, hsv.v), window, cx);
        })
}

/// The hairline that keeps a thumb visible on any colour: the model's black, a little see-through.
fn ring_shadow() -> BoxShadow {
    BoxShadow {
        color: Hsla { a: 0.4, ..model(Color::BLACK) },
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(1.0),
        inset: false,
    }
}

/// A fraction as a whole percent.
fn percent(fraction: f32) -> usize {
    (fraction * 100.0).round() as usize
}
