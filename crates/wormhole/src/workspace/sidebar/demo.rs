//! The example band the terminal welcome's sidebar step puts in the sidebar: a small band ("Example")
//! with three rows whose statuses cycle working, needs you, done, drawn with the sidebar's own band
//! and row components so the page's text can point at them ("←") and say what each means. What the
//! rows say and when they change is core's (`welcome::demo`); this draws them.
//!
//! They are not terminals and nothing treats them as one. They are not in the window's sessions, so
//! they are not in the listing (nothing numbers them ⌘1–9, nothing filters, groups or folds them),
//! not in what is saved with the window, not in `Wormhole list`, and the keys that walk the sidebar
//! (`SidebarStop`) never reach them. Drawn without a click, a hover, a menu, a drag or a focus ring;
//! VoiceOver reads them as "Example" and three example terminals, never as the person's own.
//!
//! The band lives in this window's sidebar state, so it goes with the window; it also goes the moment
//! the program says its step was left (`WelcomeOp::Demo`), the welcome finishes (`welcome_ended`) or
//! its terminal closes (`close_session`), and it is not drawn while the welcome's gate is shut or the
//! terminal's program has exited, whatever else was missed.

use wormhole_core::config::StatusPosition;
use wormhole_core::sidebar::RowState;
use wormhole_core::welcome::demo as example;

use super::rows::Frame;
use super::*;
use crate::style::{BandKind, BandParts, FolderLook, RowTone, Section, SessionRow, StatusGlyph};

/// How often the redraw timer looks again while Reduce Motion holds the rows still (it may be turned
/// off meanwhile).
const STILL_CHECK: Duration = Duration::from_secs(1);

/// The band while it is up.
pub(in crate::workspace) struct Demo {
    /// The welcome terminal whose step asked for it.
    tab: i64,
    /// Since when it has been up: the rows' states follow the time.
    since: Instant,
    /// Wakes the sidebar at each change. Dropped with the band.
    _redraw: Task<()>,
}

impl Workspace {
    /// The welcome in `tab` says its sidebar step is up (`show`) or was left. Only a terminal of this
    /// window can put the band up; asking again while it is up leaves the cycle where it is.
    pub(in crate::workspace) fn set_welcome_demo(&mut self, tab: i64, show: bool, cx: &mut Context<Self>) {
        if !show {
            self.end_welcome_demo(tab);
        } else if self.sidebar.demo.as_ref().is_none_or(|demo| demo.tab != tab)
            && self.sessions.iter().any(|session| session.id == tab)
        {
            let since = Instant::now();
            let redraw = cx.spawn(async move |this, cx| {
                loop {
                    let still = cx.update(|cx| cx.reduce_motion());
                    let wait = example::until_change(since.elapsed(), still).unwrap_or(STILL_CHECK);
                    cx.background_executor().timer(wait).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            });
            self.sidebar.demo = Some(Demo { tab, since, _redraw: redraw });
        }
        cx.notify();
    }

    /// Take the band away if it is `tab`'s.
    pub(in crate::workspace) fn end_welcome_demo(&mut self, tab: i64) {
        if self.sidebar.demo.as_ref().is_some_and(|demo| demo.tab == tab) {
            self.sidebar.demo = None;
        }
    }

    /// The band, if it is to be drawn now: asked for, its welcome still running, and its terminal's
    /// program not ended (a welcome that was killed would otherwise leave it up).
    fn demo_in_view(&self, cx: &App) -> Option<&Demo> {
        self.sidebar.demo.as_ref().filter(|demo| {
            super::super::welcome::running(demo.tab, cx)
                && self.sessions.iter().any(|session| {
                    session.id == demo.tab && session.terminal().read(cx).status() != Status::Exited
                })
        })
    }

    /// The example rows as drawn now (devctl and the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(in crate::workspace) fn welcome_demo_rows(&self, cx: &App) -> Option<[example::Row; 3]> {
        let demo = self.demo_in_view(cx)?;
        Some(example::rows(demo.since.elapsed(), cx.reduce_motion()))
    }

    /// The band and its three rows, to follow the list's own entries; none when it is not up.
    pub(super) fn render_demo(&self, frame: &Frame, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(demo) = self.demo_in_view(cx) else { return Vec::new() };
        let ui = frame.ui;
        let rows = example::rows(demo.since.elapsed(), cx.reduce_motion());
        let s = settings(cx);
        let (show_dot, dot_right) =
            (s.sidebar_status_dot, s.sidebar_status_position == StatusPosition::Right);

        // Display only: nothing to click, fold or drop on (and `display_only` takes the hover away).
        let band = style::folder_band(
            "welcome-demo-band",
            ui,
            BandParts {
                kind: BandKind::Folder,
                section: Section::Open,
                title: example::BAND.into(),
                detail: None,
                count: None,
                waiting: None,
                add: None,
                on_toggle: None,
                drop_target: false,
                sticky: false,
                joined: false,
            },
            FolderLook::default(),
            None,
        )
        .role(Role::TreeItem)
        .aria_label(example::band_label())
        .aria_expanded(true)
        .aria_level(1);

        let mut items = vec![display_only(band)];
        for (ix, row) in rows.into_iter().enumerate() {
            let glyph = style::status_glyph(("welcome-demo-status", ix as u64), ui, glyph_of(row.state), cx);
            // The status sits where the person's setting puts it, as on a real row.
            let (lead, at_right) = match (show_dot, dot_right) {
                (true, false) => (glyph, None),
                (true, true) => (div().into_any_element(), Some(glyph)),
                (false, _) => (div().into_any_element(), None),
            };
            let trail =
                at_right.map(|glyph| div().flex().items_center().gap_2().child(glyph).into_any_element());
            let tone = match row.state {
                RowState::NeedsYou => RowTone::Attention,
                RowState::Finished => RowTone::Done,
                _ => RowTone::Plain,
            };
            let detail = (!frame.compact).then(|| SharedString::from(row.detail));
            let parts = SessionRow { detail, tone, trail, ..SessionRow::new(lead, row.name) };
            items.push(display_only(
                style::session_row(("welcome-demo-row", ix as u64), ui, parts)
                    .role(Role::TreeItem)
                    .aria_label(row.spoken)
                    .aria_level(2),
            ));
        }
        items
    }
}

/// `element` for show only. The band and the row are built for the real list, with a hover fill and a
/// hand for the pointer that cannot be taken off afterwards, so a transparent layer over them takes
/// the pointer instead (it blocks what is under it): no fill, no hand, no click gets through.
fn display_only(element: impl IntoElement) -> AnyElement {
    div()
        .relative()
        .w_full()
        .child(element)
        .child(div().absolute().top_0().left_0().size_full().occlude().cursor_default())
        .into_any_element()
}

/// The glyph a state is drawn with: an agent's, as `rows::glyph_for` has it for a real row.
fn glyph_of(state: RowState) -> StatusGlyph {
    match state {
        RowState::Working => StatusGlyph::Working,
        RowState::NeedsYou => StatusGlyph::NeedsYou,
        RowState::Finished => StatusGlyph::Done,
        RowState::Failed => StatusGlyph::Failed,
        RowState::Idle | RowState::Exited => StatusGlyph::Idle,
    }
}
