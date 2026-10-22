//! The welcome page: the app's mark and name, what it is for, a few lines of the real sidebar to show
//! it, and the way in. It is what the window carries through the wormhole, so each part has its own
//! moment to come in (`wormhole_core::onboarding::intro::page`).

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use style::{RowTone, SessionRow, StatusGlyph};
use wormhole_core::onboarding::Event;
use wormhole_core::onboarding::intro::page::Entrance;
use wormhole_core::onboarding::welcome::TAGLINE;

use super::Intro;
use super::page::Entering;
use crate::icons::app_icon_for;
use crate::settings::{SettingsStore, UiColors};
use crate::style::{self, TextRole};
use crate::window::Workspace;
use crate::window::aux_window;

/// The page's width: the specimen is as wide as the sidebar's column, less the page's side padding.
const WELCOME_W: f32 = 408.0;
const MARK: f32 = 96.0;

impl Workspace {
    /// The welcome page as `entrance` has brought it in.
    pub(super) fn render_welcome(
        &self,
        intro: &Intro,
        ui: UiColors,
        entrance: Entrance,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let last = intro.model.is_last();
        let (label, event) =
            if last { ("Back to Wormhole", Event::Start) } else { ("Get started", Event::Continue) };
        div()
            .w(px(WELCOME_W))
            .max_w_full()
            .px_6()
            .flex()
            .flex_col()
            .items_center()
            .child(logo(entrance.logo, app_icon_for(cx.global::<SettingsStore>().tokens.appearance)))
            .child(
                div()
                    .mt_6()
                    .text_title()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Wormhole")
                    .entering(entrance.title),
            )
            .child(
                div()
                    .mt_1()
                    .text_body()
                    .text_color(ui.muted)
                    .text_center()
                    .child(TAGLINE)
                    .entering(entrance.tagline),
            )
            .child(
                div()
                    .mt_8()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(specimen(ui, aux_window::surfaces(cx, false).panel, cx))
                    .child(
                        div()
                            .text_caption()
                            .text_color(ui.muted)
                            .text_center()
                            .child("Which agent is working, which needs you, which is done."),
                    )
                    .entering(entrance.body),
            )
            .child(
                div()
                    .mt_8()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(Button::new("intro-start").primary().label(label).on_click(cx.listener(
                        move |this, _, window, cx| this.onboarding_event(event.clone(), window, cx),
                    )))
                    // A button, not a link: ⇥ reaches it, and VoiceOver names it.
                    .child(Button::new("intro-skip").ghost().label("Skip intro").on_click(
                        cx.listener(|this, _, window, cx| this.onboarding_event(Event::Skip, window, cx)),
                    ))
                    .entering(entrance.actions),
            )
            .child(self.render_keys_hint(ui, last).entering(entrance.actions))
            .into_any_element()
    }

    /// The keys that matter, in a quiet line under the buttons: ←, → and R work too, and the corner's
    /// button says so.
    fn render_keys_hint(&self, ui: UiColors, _last: bool) -> Div {
        let hint = |key: &'static str, what: &'static str| {
            div().flex().items_center().gap_2().child(style::keycap(ui, key)).child(div().child(what))
        };
        div()
            .mt_4()
            .flex()
            .items_center()
            .gap_4()
            .text_caption()
            .text_color(ui.muted)
            .child(hint("⏎", "continue"))
            .child(hint("esc", "skip"))
    }
}

/// The app's mark, which the ship of the voyage intro docks on: it comes in by fading, in place (the
/// ship lands exactly there), and says where it is every frame, so the ship flies to the real spot.
#[track_caller]
fn logo(progress: f32, icon: &'static str) -> Div {
    div()
        .relative()
        .size(px(MARK))
        .flex_none()
        .child(
            canvas(
                |bounds, _, cx| {
                    let centre = |origin: Pixels, size: Pixels| f32::from(origin + size / 2.0);
                    crate::intro::set_landing(
                        cx,
                        [
                            centre(bounds.origin.x, bounds.size.width),
                            centre(bounds.origin.y, bounds.size.height),
                            f32::from(bounds.size.width),
                        ],
                    );
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .child(img(icon).size(px(MARK)).opacity(progress))
}

/// A few lines of the real sidebar, in the three states an agent is in: what the app is for, shown
/// rather than told. It is drawn with the sidebar's own components, so it follows the theme and is
/// what the app looks like. Nothing in it reacts: a plate over it keeps the pointer off its rows.
#[track_caller]
fn specimen(ui: UiColors, panel: Hsla, cx: &App) -> Div {
    let metrics = style::Metrics::default();
    let row = |id: &'static str,
               glyph: StatusGlyph,
               name: &'static str,
               detail: &'static str,
               tone: RowTone,
               selected: bool| {
        style::session_row(
            id,
            ui,
            SessionRow {
                detail: Some(detail.into()),
                tone,
                selected,
                tile: Some(style::status_tile_at(
                    format!("{id}-tile"),
                    ui,
                    style::StatusTile { face: style::TileFace::Terminal, glyph, selected, mini: false },
                    metrics.tile,
                    cx,
                )),
                card: true,
                metrics,
                ..SessionRow::new(div().into_any_element(), name)
            },
        )
    };
    let head = style::group_head(
        "welcome-band",
        ui,
        style::GroupHead {
            metrics,
            name: "api-server".into(),
            tooltip: None,
            folder: None,
            color: None,
            folded: false,
            pinned: false,
            tiles: Vec::new(),
            count: 3,
            alert: None,
            empty: None,
            add: None,
            on_toggle: Box::new(|_, _, _| {}),
            spoken_edit: SharedString::default(),
            spoken_fold: SharedString::default(),
            on_label: None,
            label_anchor: None,
            name_edit: None,
        },
    );
    style::palette_surface(ui, true)
        .relative()
        .w_full()
        .bg(panel)
        .p_1()
        .child(head)
        .child(row(
            "welcome-working",
            StatusGlyph::Working,
            "auth refactor",
            "Editing middleware.rs…",
            RowTone::Plain,
            true,
        ))
        .child(row(
            "welcome-needs-you",
            StatusGlyph::NeedsYou,
            "release notes",
            "Needs you · Approve git push?",
            RowTone::Attention,
            false,
        ))
        .child(row(
            "welcome-done",
            StatusGlyph::Done,
            "api tests",
            "Finished · 12m ago",
            RowTone::Done,
            false,
        ))
        .child(div().absolute().top_0().left_0().size_full().occlude())
}
