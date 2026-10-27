//! The window sketch and the choice card it is offered in: the three presets of the first run's walk,
//! each as the window it makes, the second chosen.

use gpui_kit::*;
use wormhole_core::config::presets::{Preset, Shown};

use super::Env;
use super::view::GalleryView;
use crate::style::{self, Edges, TextRole};

/// The names under the sketches; the Studio's gallery is not interface text.
const NAMES: [&str; 3] = ["Zen", "Balanced", "Everything"];
/// The tabs "all of the tools" draws in a sketch.
const TOOLS: usize = 4;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    div()
        .flex()
        .gap_2()
        .w_full()
        .children(Preset::ALL.iter().zip(NAMES).enumerate().map(|(ix, (preset, name))| {
            style::choice_card(("g-sketch-card", ix), ui, ix == 1)
                .flex_1()
                .child(div().p_1().child(style::window_sketch(ui, &Shown::of(*preset), TOOLS)))
                .child(
                    div()
                        .p_1()
                        .rule_top(ui)
                        .text_caption()
                        .text_color(if ix == 1 { ui.primary } else { ui.text })
                        .child(name),
                )
        }))
        .into_any_element()
}
