//! Menu bar's preview: two stretches of the menu bar with Wormhole's item as the settings make it, one
//! while a terminal needs input (the dot and the count that "Icon shows" chooses) and one while a
//! terminal works (the filled icon, whose prompt types itself out when "Animate" is on). With "Show
//! Wormhole in the menu bar" off the item is not there.
//!
//! What the item says is the app's own rule (`menubar::model::status_icon`, from a sample terminal) and
//! its animation runs on the app's own frames (`notify::MENU_ANIM_FRAMES`); only the drawing is this
//! file's, from the parts the glyph has (`platform/macos/status_item/icon.rs`): the box, the prompt and
//! the badge. It is a view of its own so that its clock, a frame every `MENU_ANIM_FRAME` while the prompt
//! types, is not the window's. Reduce Motion holds the prompt still, as it does in the menu bar.

use std::time::{Duration, Instant};

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::notify::{ItemState, MENU_ANIM_FRAME, MENU_ANIM_FRAMES, Source, State, Terminal};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::menubar::model::{Mark, StatusIcon, status_icon};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::ui;
use crate::style::{self, Edges, TextRole};

/// The menu bar's height.
const MENU_BAR_H: f32 = 24.0;
/// The glyph's box, in points (the menu bar's own is 13.5 by 11; this one holds the Kit's chevron).
const BOX_W: f32 = 22.0;
const BOX_H: f32 = 16.0;
/// The prompt's line grows by this much a frame, up to the three frames of the full line.
const LINE_STEP: f32 = 2.0;
/// The badge of a state; its ring is the theme's border.
const BADGE: f32 = 8.0;
/// How often a still preview looks again whether it may move: the settings it follows notify by
/// themselves, Reduce Motion is the one fact that has no event here.
const STILL_CHECK: Duration = Duration::from_secs(1);

/// The menu bar, twice: while a terminal needs input and while one works.
pub(in crate::settings_window) struct MenuBarPreview {
    /// When the animation began: its frame is the time since, in frames.
    began: Instant,
    /// Redraws the preview a frame at a time while the prompt types; dropped with it.
    _clock: Task<()>,
    /// A setting changed, or the theme did: draw again.
    _settings: Subscription,
}

/// Whether the item is there and its prompt types: the switch, "Animate", and not Reduce Motion.
fn moves(cx: &App) -> bool {
    let settings = settings(cx);
    settings.menu_bar && settings.menu_bar_animate && !cx.reduce_motion()
}

impl MenuBarPreview {
    pub(in crate::settings_window) fn new(cx: &mut Context<Self>) -> Self {
        let clock = cx.spawn(async move |this, cx| {
            loop {
                let moving = cx.update(|cx| moves(cx));
                cx.background_executor().timer(if moving { MENU_ANIM_FRAME } else { STILL_CHECK }).await;
                if this
                    .update(cx, |_, cx| {
                        if moving {
                            cx.notify()
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            began: Instant::now(),
            _clock: clock,
            _settings: cx.observe_global::<SettingsStore>(|_, cx| cx.notify()),
        }
    }

    /// The prompt's frame now, or none while it holds still (the full prompt).
    fn frame(&self, cx: &App) -> Option<usize> {
        moves(cx).then(|| {
            let frames = self.began.elapsed().as_millis() / MENU_ANIM_FRAME.as_millis().max(1);
            // A handful of frames: the remainder fits.
            (frames % MENU_ANIM_FRAMES as u128) as usize
        })
    }
}

/// A sample terminal in `state`: what the item says of it is the app's own rule.
fn terminal(state: State) -> Terminal {
    Terminal {
        session: 0,
        tab: 0,
        name: String::new(),
        state,
        since: Instant::now(),
        source: Source::Agent,
        who: None,
        message: None,
        command: None,
        took: None,
        exit: None,
        changed: None,
        folder: None,
        branch: None,
    }
}

impl Render for MenuBarPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let settings = settings(cx);
        let (shown, mode, animate) = (settings.menu_bar, settings.menu_bar_icon, moves(cx));
        let frame = self.frame(cx);
        let bar = |label: &'static str, state: State| {
            let icon = status_icon(&[terminal(state)], mode, animate);
            menu_bar(ui, label, shown.then_some((icon, frame)))
        };
        style::preview_frame(ui)
            .flex()
            .flex_col()
            .gap_2()
            .child(bar(text(SETTINGS_MENU_BAR_PREVIEW_NEEDS_INPUT), State::NeedsYou))
            .child(bar(text(SETTINGS_MENU_BAR_PREVIEW_WORKING), State::Working))
    }
}

/// A stretch of the menu bar: `label` says which state it shows, then the clock's side, with Wormhole's
/// item among the others while it is shown.
fn menu_bar(ui: UiColors, label: &'static str, item: Option<(StatusIcon, Option<usize>)>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .h(px(MENU_BAR_H))
        .px_3()
        .rounded(ui.radius)
        .bg(ui.panel)
        .outlined(ui, ui.border)
        .text_caption()
        .child(div().flex_1().min_w_0().truncate().text_color(ui.faint).child(label))
        .children(item.map(|(icon, frame)| status_item(ui, &icon, frame)))
        .child(style::icon(IconName::Search).text_color(ui.muted))
        .child(div().text_color(ui.text).child("Sun 18:42")) // text: sample (a made-up clock of the preview)
}

/// Wormhole's item: the terminal glyph, filled while something works, with the state's badge on its
/// corner and the count beside it.
fn status_item(ui: UiColors, icon: &StatusIcon, frame: Option<usize>) -> Div {
    let badge = match icon.mark {
        Mark::None => None,
        Mark::NeedsYou => Some(ui.agent_attention),
        Mark::Finished => Some(ui.agent_done),
        Mark::Failed => Some(ui.danger),
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(glyph(ui, icon.state == ItemState::Working, frame, badge))
        .children(icon.count.clone().map(|count| {
            div().font_weight(FontWeight::SEMIBOLD).tabular_nums().text_color(ui.text).child(count)
        }))
}

/// The terminal: a box with the `>` prompt and its line. While working the box is filled and the prompt
/// cut out of it (drawn in the bar's colour); a `frame` draws the line that many steps long (none: full).
fn glyph(ui: UiColors, filled: bool, frame: Option<usize>, badge: Option<Hsla>) -> Div {
    let ink = if filled { ui.panel } else { ui.text };
    let line = frame.unwrap_or(MENU_ANIM_FRAMES - 1) as f32 * LINE_STEP;
    div()
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .w(px(BOX_W))
        .h(px(BOX_H))
        .pl_1()
        .rounded(ui.radius_sm)
        .outlined(ui, ui.text)
        .when(filled, |glyph| glyph.bg(ui.text))
        .child(style::icon(IconName::ChevronRight).xsmall().text_color(ink))
        .child(div().flex_none().w(px(line)).h(ui.border_width).bg(ink))
        .children(badge.map(|color| {
            div()
                .absolute()
                .top(-px(BADGE / 2.0))
                .right(-px(BADGE / 2.0))
                .size(px(BADGE))
                .rounded_full()
                .bg(color)
                .outlined(ui, ui.panel)
        }))
}
