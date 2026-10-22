//! What opens over the window's content: the command palette and the keyboard-shortcuts
//! sheet, one at a time, each on a scrim that takes the clicks. Closing one gives the
//! keyboard back to whatever had it when it opened ("focus returns where it was").

use gpui_kit::*;

use crate::settings::SettingsStore;
use crate::style;
use crate::theme_bridge::token;
use crate::workspace::Workspace;

/// Which overlay is open.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayKind {
    Palette,
    Sheet,
}

impl OverlayKind {
    /// What VoiceOver calls the dialog.
    fn name(self) -> &'static str {
        match self {
            OverlayKind::Palette => "Command palette",
            OverlayKind::Sheet => "Keyboard shortcuts",
        }
    }
}

/// The overlay's view, kept as an `AnyView` so the window draws it without naming the palette's or
/// the sheet's type (they sit above the window); who needs the entity downcasts it ([`Workspace::overlay_of`]).
pub(crate) struct OverlayView {
    kind: OverlayKind,
    view: AnyView,
}

/// A view that can be put over the window. Its kind is part of the type, so a caller cannot hand
/// over a view under another view's kind.
pub(crate) trait OverlayContent: Render {
    const KIND: OverlayKind;
}

impl OverlayView {
    /// The overlay for `view`, of the kind its type says.
    pub(crate) fn new<V: OverlayContent>(view: Entity<V>) -> Self {
        Self { kind: V::KIND, view: view.into() }
    }
}

pub(super) struct Overlay {
    view: OverlayView,
    return_focus: ReturnFocus,
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
        if let Some(open) = self.overlay.take() {
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
        self.overlay = Some(Overlay { view, return_focus });
        cx.notify();
    }

    /// Take the overlay down and give the keyboard back.
    pub(crate) fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(overlay) = self.overlay.take() else { return };
        let ReturnFocus { handle, tab } = overlay.return_focus;
        let gone = tab.is_some_and(|tab| self.find_session(tab).is_none());
        match handle.filter(|_| !gone) {
            Some(focus) => window.focus(&focus, cx),
            None => window.focus(&self.active_terminal().focus_handle(cx), cx),
        }
        cx.notify();
    }

    fn overlay_is(&self, kind: OverlayKind) -> bool {
        self.overlay.as_ref().is_some_and(|o| o.view.kind == kind)
    }

    pub(crate) fn palette_open(&self) -> bool {
        self.overlay_is(OverlayKind::Palette)
    }

    pub(crate) fn sheet_open(&self) -> bool {
        self.overlay_is(OverlayKind::Sheet)
    }

    /// Whether the first-run sheet is up: it holds the keyboard until it is done, so no overlay
    /// opens over it.
    pub(crate) fn first_run_open(&self) -> bool {
        self.first_run.is_open()
    }

    /// The open overlay as a `T`, if it is one (devctl types into the palette and the sheet).
    #[cfg(feature = "devctl")]
    pub(crate) fn overlay_of<T: 'static>(&self) -> Option<Entity<T>> {
        self.overlay.as_ref()?.view.view.clone().downcast::<T>().ok()
    }

    /// The overlay over everything else in the window.
    pub(super) fn render_overlay(&self) -> Option<AnyElement> {
        let open = &self.overlay.as_ref()?.view;
        let (view, name) = (open.view.clone().into_any_element(), open.kind.name());
        // The scrim and the card fade in together, in `MOTION_POPOVER`; they come in mounted,
        // so each opening starts it again. Under Reduce Motion GPUI draws the last frame: a cut.
        Some(
            div()
                .id("overlay")
                // A dialog VoiceOver names.
                .role(Role::Dialog)
                .aria_label(name)
                .absolute()
                .size_full()
                .child(view)
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

/// The scrim under an overlay (`surface.overlay`), the whole window: it keeps the pointer from
/// what is under it. The card goes in with `.child(…)`, placed by the caller, and closes on a
/// click outside it (`on_mouse_down_out`).
#[track_caller]
pub(crate) fn scrim(id: &'static str, cx: &App) -> Stateful<Div> {
    let color = token(&cx.global::<SettingsStore>().tokens, "surface.overlay");
    div().id(id).absolute().top_0().left_0().size_full().bg(color).occlude()
}
