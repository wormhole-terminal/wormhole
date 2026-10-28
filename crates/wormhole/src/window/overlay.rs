//! What opens over the window's content: the command palette and the keyboard-shortcuts
//! sheet, one at a time, each on a scrim that takes the clicks, and the folder picker, which has nothing of
//! its own to draw: its panel is the system's, a window beside this one (a click outside it closes it and reaches
//! what it lands on). Closing one gives the keyboard back to whatever had it when it opened ("focus returns where
//! it was").
//!
//! With the Motion feature on the palette appears at once and fades out in [`EXIT`] when it closes (core
//! `view::motion::overlay`). It is closed from the moment it is told to be: the keyboard is already where it
//! goes back to and the window no longer counts it open, so it takes no key. What fades is its last picture,
//! the same entity drawn over a shield that takes the pointer, until a timer takes it down. Without the
//! feature, or under Reduce Motion, it is as it was: it fades in over [`MOTION_POPOVER`] and closing is a cut.

use gpui_kit::*;
use wormhole_core::view::motion::overlay::{EXIT, exit_opacity};

use crate::settings::{SettingsStore, settings_in};
use crate::style;
use crate::style::motion::LAST_FRAME_GRACE;
use crate::theme_bridge::token;
use crate::window::Workspace;

/// Which overlay is open.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayKind {
    Palette,
    Sheet,
    FolderPicker,
}

impl OverlayKind {
    /// What VoiceOver calls the dialog.
    fn name(self) -> &'static str {
        match self {
            OverlayKind::Palette => "Command palette",
            OverlayKind::Sheet => "Keyboard shortcuts",
            OverlayKind::FolderPicker => wormhole_text::text(wormhole_text::keys::FOLDER_PICKER_NAME),
        }
    }
}

/// The overlay's view, kept as an `AnyView` so the window draws it without naming the palette's or
/// the sheet's type (they sit above the window); who needs the entity downcasts it ([`Workspace::overlay_of`]).
pub(crate) struct OverlayView {
    kind: OverlayKind,
    view: AnyView,
    /// What the view says of its card and of being closed (`OverlayContent`), through the entity it was made from.
    card: Box<dyn Fn(&App) -> Card>,
    leave: Box<dyn Fn(&mut App)>,
}

/// Where an overlay's card was last painted, if it has one.
type Card = Option<Bounds<Pixels>>;

/// A view that can be put over the window. Its kind is part of the type, so a caller cannot hand
/// over a view under another view's kind.
pub(crate) trait OverlayContent: Render {
    const KIND: OverlayKind;

    /// The card on the scrim, as last painted, in the window's coordinates: what a picture of the view that
    /// fades out keeps the pointer from. `None`: it has no card to speak of.
    fn card(&self) -> Option<Bounds<Pixels>> {
        None
    }

    /// The overlay was closed and its picture is fading out: it takes no key and no pointer of its own now.
    fn leave(&mut self, _cx: &mut Context<Self>) {}
}

impl OverlayView {
    /// The overlay for `view`, of the kind its type says.
    pub(crate) fn new<V: OverlayContent>(view: Entity<V>) -> Self {
        let (card, leave) = (view.clone(), view.clone());
        Self {
            kind: V::KIND,
            view: view.into(),
            card: Box::new(move |cx| card.read(cx).card()),
            leave: Box::new(move |cx| leave.update(cx, |view, cx| view.leave(cx))),
        }
    }
}

pub(super) struct Overlay {
    view: OverlayView,
    return_focus: ReturnFocus,
}

/// A palette that was closed and is fading out.
struct Leaving {
    view: OverlayView,
    /// Where its card was, which the shield covers.
    card: Option<Bounds<Pixels>>,
    /// Which exit it is, so a later one is not ended by an earlier one's timer.
    serial: u64,
    _end: Task<()>,
}

/// What is over the window: the overlay that is open, or the one that was just closed and is fading. Never
/// both: an overlay that opens takes down the one that fades.
#[derive(Default)]
pub(super) struct Overlays {
    open: Option<Overlay>,
    leaving: Option<Leaving>,
    exits: u64,
}

impl Overlays {
    /// Whether an overlay is open: not one that is fading, which has no keys and is no longer there as far
    /// as the window's state goes.
    pub(super) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Whether something is drawn over the window's content: an overlay that is open, or one fading out
    /// (which still takes the pointer). A native panel over the same place would take clicks from it.
    pub(super) fn is_shown(&self) -> bool {
        self.open.is_some() || self.leaving.is_some()
    }

    /// Whether a palette is on its way out, drawn and taking nothing.
    #[cfg(test)]
    pub(super) fn is_leaving(&self) -> bool {
        self.leaving.is_some()
    }
}

/// What had the keyboard before the first overlay opened.
pub(crate) struct ReturnFocus {
    handle: Option<FocusHandle>,
    /// The terminal it was, if it was one: one closed meanwhile (the palette's ⌘⌫) takes its focus
    /// with it, and the current terminal gets the keyboard instead.
    tab: Option<i64>,
}

impl Workspace {
    /// Before a new overlay is made (it takes the keyboard): take down the one already there and
    /// say where the keyboard goes back to when the new one closes, which is where it was before
    /// the first.
    pub(crate) fn end_overlay(&mut self, window: &mut Window, cx: &App) -> ReturnFocus {
        self.overlay.leaving = None;
        if let Some(open) = self.overlay.open.take() {
            return open.return_focus;
        }
        let handle = window.focused(cx);
        let tab = handle.as_ref().and_then(|handle| {
            self.sessions
                .iter()
                .find(|session| session.terminal().focus_handle(cx) == *handle)
                .map(|session| session.id)
        });
        ReturnFocus { handle, tab }
    }

    /// Put `view` over the window; closing it gives the keyboard back as `return_focus` says.
    pub(crate) fn show_overlay(
        &mut self,
        view: OverlayView,
        return_focus: ReturnFocus,
        cx: &mut Context<Self>,
    ) {
        self.overlay.leaving = None;
        self.overlay.open = Some(Overlay { view, return_focus });
        cx.notify();
    }

    /// Take the overlay down and give the keyboard back. A palette fades out first, when motion is on.
    pub(crate) fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(overlay) = self.overlay.open.take() else { return };
        let ReturnFocus { handle, tab } = overlay.return_focus;
        let gone = tab.is_some_and(|tab| self.find_session(tab).is_none());
        match handle.filter(|_| !gone) {
            Some(focus) => window.focus(&focus, cx),
            None => window.focus(&self.active_terminal().focus_handle(cx), cx),
        }
        if overlay.view.kind == OverlayKind::Palette && self.fades_palette(cx) {
            // After the current update: the palette's own key handler and its click-outside close it from
            // inside its own update, and its card and its `leave` are read and written through that entity.
            cx.defer_in(window, move |this, _, cx| {
                // Something else opened meanwhile and took the fading one down with it.
                if this.overlay.open.is_some() {
                    return;
                }
                (overlay.view.leave)(cx);
                this.fade_out(overlay.view, cx);
                cx.notify();
            });
        }
        cx.notify();
    }

    /// Whether the palette appears at once and leaves by a fade: the Motion feature's, under no Reduce Motion.
    fn fades_palette(&self, cx: &App) -> bool {
        self.plays_motion(cx)
    }

    /// Keep `view` drawn, fading, for [`EXIT`] and then take it down.
    fn fade_out(&mut self, view: OverlayView, cx: &mut Context<Self>) {
        let serial = self.overlay.exits + 1;
        self.overlay.exits = serial;
        let end = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(EXIT + LAST_FRAME_GRACE).await;
            this.update(cx, |this, cx| {
                if this.overlay.leaving.as_ref().is_some_and(|leaving| leaving.serial == serial) {
                    this.overlay.leaving = None;
                    cx.notify();
                }
            })
            .ok();
        });
        let card = (view.card)(cx);
        self.overlay.leaving = Some(Leaving { view, card, serial, _end: end });
    }

    fn overlay_is(&self, kind: OverlayKind) -> bool {
        self.overlay.open.as_ref().is_some_and(|o| o.view.kind == kind)
    }

    pub(crate) fn palette_open(&self) -> bool {
        self.overlay_is(OverlayKind::Palette)
    }

    pub(crate) fn sheet_open(&self) -> bool {
        self.overlay_is(OverlayKind::Sheet)
    }

    pub(crate) fn folder_picker_open(&self) -> bool {
        self.overlay_is(OverlayKind::FolderPicker)
    }

    /// Whether the first-run sheet is up: it holds the keyboard until it is done, so no overlay
    /// opens over it.
    pub(crate) fn first_run_open(&self, cx: &App) -> bool {
        self.first_run.read(cx).is_open()
    }

    /// Whether an overlay that works on the terminal in front may open: the first-run sheet isn't up and no
    /// diff covers the terminal (there is no cursor to put anything at).
    pub(crate) fn terminal_overlay_allowed(&self, cx: &App) -> bool {
        !self.first_run_open(cx) && !self.covered(cx)
    }

    /// The open overlay as a `T`, if it is one (devctl types into the palette and the sheet).
    #[cfg(feature = "devctl")]
    pub(crate) fn overlay_of<T: 'static>(&self) -> Option<Entity<T>> {
        self.overlay.open.as_ref()?.view.view.clone().downcast::<T>().ok()
    }

    /// The overlay over everything else in the window: the open one, or a palette fading out.
    pub(super) fn render_overlay(&self, cx: &App) -> Option<AnyElement> {
        if let Some(Leaving { view, card, serial, .. }) = &self.overlay.leaving {
            return Some(fading(view, *card, *serial));
        }
        let open = &self.overlay.open.as_ref()?.view;
        // The folder picker is a native panel: the overlay only holds its place.
        if open.kind == OverlayKind::FolderPicker {
            return None;
        }
        let (view, name) = (open.view.clone().into_any_element(), open.kind.name());
        let overlay = div()
            .id("overlay")
            .debug_selector(|| "overlay".into())
            // A dialog VoiceOver names.
            .role(Role::Dialog)
            .aria_label(name)
            .absolute()
            .size_full()
            .child(view);
        // The scrim and the card fade in together, in `MOTION_POPOVER`; they come in mounted,
        // so each opening starts it again. Under Reduce Motion GPUI draws the last frame: a cut. The palette
        // with motion on does not fade in: it is there as the keys are pressed.
        if !fades_in(open.kind, settings_in(&self.profile, cx).motion_feature_on()) {
            return Some(overlay.into_any_element());
        }
        Some(
            overlay
                .with_animation(
                    "overlay-in",
                    Animation::new(style::motion::MOTION_POPOVER)
                        .with_easing(style::motion::EASE_OUT.easing()),
                    |overlay, t| overlay.opacity(t),
                )
                .into_any_element(),
        )
    }
}

/// Whether an overlay fades in as it opens: the sheet, and the palette unless motion is on, when it is there at
/// once. The folder picker is a popover at the cursor, there as the key is pressed.
fn fades_in(kind: OverlayKind, motion_on: bool) -> bool {
    match kind {
        OverlayKind::Palette => !motion_on,
        OverlayKind::Sheet => true,
        OverlayKind::FolderPicker => false,
    }
}

/// A palette that was closed, drawn fading: its scrim and card as they were. The card is under a shield that
/// takes the pointer, so nothing in it is live; the scrim is only a picture (`OverlayContent::leave` makes it
/// one), so a click outside the card, on what is under it, lands there.
fn fading(view: &OverlayView, card: Option<Bounds<Pixels>>, serial: u64) -> AnyElement {
    let shield = div().id("overlay-shield").absolute().occlude().debug_selector(|| "overlay-shield".into());
    let shield = match card {
        Some(card) => shield.left(card.origin.x).top(card.origin.y).w(card.size.width).h(card.size.height),
        None => shield.size_full(),
    };
    div()
        .id("overlay-leaving")
        .absolute()
        .size_full()
        .child(view.view.clone())
        .child(shield)
        .with_animation(("overlay-out", serial), Animation::new(EXIT), |overlay, t| {
            overlay.opacity(exit_opacity(t))
        })
        .into_any_element()
}

/// The scrim under an overlay (`surface.overlay`), the whole window: it keeps the pointer from
/// what is under it. The card goes in with `.child(…)`, placed by the caller, and closes on a
/// click outside it (`on_mouse_down_out`).
#[track_caller]
pub(crate) fn scrim(id: &'static str, cx: &App) -> Stateful<Div> {
    scrim_surface(id, cx).occlude()
}

/// The scrim's colour over the window and nothing else: a picture of one, which keeps nothing from what is
/// under it.
#[track_caller]
pub(crate) fn scrim_surface(id: &'static str, cx: &App) -> Stateful<Div> {
    let color = token(&cx.global::<SettingsStore>().tokens, "surface.overlay");
    div().id(id).absolute().top_0().left_0().size_full().bg(color)
}
