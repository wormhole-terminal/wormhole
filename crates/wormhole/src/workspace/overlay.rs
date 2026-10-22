//! What opens over the window's content: the command palette and the keyboard-shortcuts
//! sheet, one at a time, each on a scrim that takes the clicks. Closing one gives the
//! keyboard back to whatever had it when it opened ("focus returns where it was").

use gpui_kit::*;

use super::Workspace;
use super::command_palette::CommandPalette;
use super::shortcut_sheet::ShortcutSheet;
use crate::settings::SettingsStore;
use crate::style;
use crate::theme_bridge::token;

/// Which overlay is open.
pub(super) enum OverlayView {
    Palette(Entity<CommandPalette>),
    Sheet(Entity<ShortcutSheet>),
}

pub(super) struct Overlay {
    pub view: OverlayView,
    return_focus: ReturnFocus,
}

/// What had the keyboard before the first overlay opened.
pub(super) struct ReturnFocus {
    handle: Option<FocusHandle>,
    /// The terminal it was, if it was one: one closed meanwhile (the palette's ⌘⌫) takes its focus
    /// with it, and the current terminal gets the keyboard instead.
    tab: Option<i64>,
}

impl Workspace {
    /// Before a new overlay is made (it takes the keyboard): take down the one already there and
    /// say where the keyboard goes back to when the new one closes, which is where it was before
    /// the first.
    pub(super) fn end_overlay(&mut self, window: &mut Window, cx: &App) -> ReturnFocus {
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
    pub(super) fn show_overlay(
        &mut self,
        view: OverlayView,
        return_focus: ReturnFocus,
        cx: &mut Context<Self>,
    ) {
        self.overlay = Some(Overlay { view, return_focus });
        cx.notify();
    }

    /// Take the overlay down and give the keyboard back.
    pub(super) fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(overlay) = self.overlay.take() else { return };
        let ReturnFocus { handle, tab } = overlay.return_focus;
        let gone = tab.is_some_and(|tab| self.find_session(tab).is_none());
        match handle.filter(|_| !gone) {
            Some(focus) => window.focus(&focus, cx),
            None => window.focus(&self.active_terminal().focus_handle(cx), cx),
        }
        cx.notify();
    }

    pub(super) fn palette_open(&self) -> bool {
        matches!(self.overlay.as_ref().map(|o| &o.view), Some(OverlayView::Palette(_)))
    }

    pub(super) fn sheet_open(&self) -> bool {
        matches!(self.overlay.as_ref().map(|o| &o.view), Some(OverlayView::Sheet(_)))
    }

    /// The open palette, if any (devctl types into it).
    #[cfg(feature = "devctl")]
    pub(crate) fn palette(&self) -> Option<Entity<CommandPalette>> {
        match self.overlay.as_ref().map(|o| &o.view) {
            Some(OverlayView::Palette(palette)) => Some(palette.clone()),
            _ => None,
        }
    }

    /// The open sheet, if any (devctl types into it).
    #[cfg(feature = "devctl")]
    pub(crate) fn sheet(&self) -> Option<Entity<ShortcutSheet>> {
        match self.overlay.as_ref().map(|o| &o.view) {
            Some(OverlayView::Sheet(sheet)) => Some(sheet.clone()),
            _ => None,
        }
    }

    /// The overlay over everything else in the window.
    pub(super) fn render_overlay(&self) -> Option<AnyElement> {
        let (view, name) = match &self.overlay.as_ref()?.view {
            OverlayView::Palette(palette) => (palette.clone().into_any_element(), "Command palette"),
            OverlayView::Sheet(sheet) => (sheet.clone().into_any_element(), "Keyboard shortcuts"),
        };
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
pub(super) fn scrim(id: &'static str, cx: &App) -> Stateful<Div> {
    let color = token(&cx.global::<SettingsStore>().tokens, "surface.overlay");
    div().id(id).absolute().top_0().left_0().size_full().bg(color).occlude()
}
