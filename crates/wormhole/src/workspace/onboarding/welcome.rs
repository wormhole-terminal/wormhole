//! The welcome page: the app's mark and name, what it is for, a few lines of the real sidebar to show
//! it, and the way in. It is what the window carries through the wormhole, so each part has its own
//! moment to come in (`wormhole_core::intro::page`).

use style::{BandKind, BandParts, FolderLook, FolderMeta, RowTone, Section, SessionRow, StatusGlyph};
use wormhole_core::intro::page::Entrance;

use super::page::Entering;
use super::*;
use crate::icons::APP_ICON;
use crate::workspace::about::TAGLINE;
use crate::workspace::aux_window;

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
            .child(logo(entrance.logo))
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
fn logo(progress: f32) -> Div {
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
        .child(img(APP_ICON).size(px(MARK)).opacity(progress))
}

/// A few lines of the real sidebar, in the three states an agent is in: what the app is for, shown
/// rather than told. It is drawn with the sidebar's own components, so it follows the theme and is
/// what the app looks like. Nothing in it reacts: a plate over it keeps the pointer off its rows.
#[track_caller]
fn specimen(ui: UiColors, panel: Hsla, cx: &App) -> Div {
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
                ..SessionRow::new(style::status_glyph(format!("{id}-glyph"), ui, glyph, cx), name)
            },
        )
    };
    let band = style::folder_band(
        "welcome-band",
        ui,
        BandParts {
            kind: BandKind::Folder,
            section: Section::Open,
            title: "api-server".into(),
            detail: None,
            count: Some("3".into()),
            add: None,
            on_toggle: None,
            drop_target: false,
            sticky: false,
            // The card's own outline is its top edge: the band's hairline would show past its rounded
            // corners.
            joined: true,
        },
        FolderLook::default(),
        Some(FolderMeta {
            path: "~/Projects/api-server".into(),
            path_w: None,
            changes: Some(3),
            branch: Some(style::branch_tag(ui, Some("feature/auth")).into_any_element()),
        }),
    );
    style::palette_surface(ui, true)
        .relative()
        .w_full()
        .bg(panel)
        // Its fill is square, so it takes the card's corners itself.
        .child(band.rounded_t(ui.radius_lg))
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
