//! Hotkey's preview: the shortcut as key caps, and what pressing it opens. "Quick panel" shows the quick
//! panel (its search row and one result: two rows are all that fit the strip whole); "Show or hide
//! Wormhole" shows a window that comes and goes. The window is the first run's window sketch
//! (`style::window_sketch`) fading in and out, held shown under Reduce Motion.
//!
//! A view of its own so that the clock of the window coming and going is not the window's. The quick
//! panel is still, so it asks for no clock: only the window's loop does.

use std::time::{Duration, Instant};

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::HotkeyAction;
use wormhole_core::config::presets::{Preset, Shown};
use wormhole_core::keys::keyspec::KeySpec;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::settings::{SettingsStore, settings};
use crate::settings_window::recorder::caps;
use crate::settings_window::ui;
use crate::style::{self, StatusGlyph};

/// The window sketch's width: a window's proportions at about the height of the quick panel.
const WINDOW_W: f32 = 160.0;
/// The tabs a sketch of "all of the tools" would have; this one is the plain window.
const TOOLS: usize = 0;
/// How long the window stays shown, and gone, in a loop; the window is `HIDDEN_OPACITY` there.
const SHOWN_FOR: Duration = Duration::from_millis(1500);
const HIDDEN_FOR: Duration = Duration::from_millis(1500);
const HIDDEN_OPACITY: f32 = 0.2;
/// How often the loop looks at the clock, and a still preview at Reduce Motion.
const TICK: Duration = Duration::from_millis(250);
const STILL_CHECK: Duration = Duration::from_secs(1);

/// What the hotkey opens, and the keys that open it.
pub(in crate::settings_window) struct HotkeyPreview {
    began: Instant,
    _clock: Task<()>,
    _settings: Subscription,
}

/// Whether the window's loop runs: the hotkey shows or hides Wormhole, and motion is allowed.
fn moves(cx: &App) -> bool {
    settings(cx).hotkey_action == HotkeyAction::ToggleApp && !cx.reduce_motion()
}

impl HotkeyPreview {
    pub(in crate::settings_window) fn new(cx: &mut Context<Self>) -> Self {
        let clock = cx.spawn(async move |this, cx| {
            loop {
                let moving = cx.update(|cx| moves(cx));
                cx.background_executor().timer(if moving { TICK } else { STILL_CHECK }).await;
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

    /// Whether the window is shown at this moment of its loop (always, while it holds still).
    fn window_shown(&self, cx: &App) -> bool {
        if !moves(cx) {
            return true;
        }
        let cycle = SHOWN_FOR + HIDDEN_FOR;
        let at = self.began.elapsed().as_millis() % cycle.as_millis();
        at < SHOWN_FOR.as_millis()
    }
}

impl Render for HotkeyPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let settings = settings(cx);
        let spec = settings.hotkey.trim().parse::<KeySpec>().ok();
        let opens = match settings.hotkey_action {
            HotkeyAction::QuickPanel => quick_panel(cx),
            HotkeyAction::ToggleApp => {
                let window = div().w(px(WINDOW_W)).child(style::window_sketch(
                    ui,
                    &Shown::of(Preset::Balanced),
                    TOOLS,
                ));
                if self.window_shown(cx) { window } else { window.opacity(HIDDEN_OPACITY) }.into_any_element()
            }
        };
        // With no shortcut set there are no keys to show, and nothing leads to what it opens.
        let keys =
            spec.map(|spec| {
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(div().flex().items_center().gap_1().children(
                        caps(spec.mods, Some(&spec.key)).into_iter().map(|cap| style::keycap(ui, cap)),
                    ))
                    .child(style::icon(IconName::ArrowRight).text_color(ui.muted))
            });
        div().flex().items_center().justify_center().gap_4().children(keys).child(opens)
    }
}

/// The quick panel as the hotkey opens it: the search field and the terminal that needs input first.
fn quick_panel(cx: &App) -> AnyElement {
    let ui = ui(cx);
    let mut needs = style::PaletteResult::new(
        style::status_glyph("hotkey-preview-needs", ui, StatusGlyph::NeedsYou, cx),
        "api", // text: sample (a made-up terminal of the preview)
    );
    needs.strong = true;
    needs.detail = Some("Allow edit?".into()); // text: sample (a made-up terminal of the preview)
    needs.detail_color = Some(ui.attention_text);
    style::palette_surface(ui, false)
        .flex_1()
        .min_w_0()
        .max_w(px(style::QUICK_PANEL_W))
        .child(
            style::palette_search(ui).child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(ui.faint)
                    .child(text(SETTINGS_HOTKEY_PREVIEW_SEARCH)),
            ),
        )
        .child(style::palette_result("hotkey-preview-needs-row", ui, true, needs))
        .into_any_element()
}
