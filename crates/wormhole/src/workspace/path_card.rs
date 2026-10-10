//! The path's hover card: resting on the folder in the title row for a
//! moment opens a card under it with the whole path, wrapped at its slashes, and three actions:
//! Open in Finder, Copy path, and Insert into terminal, which types the path, quoted for the shell,
//! at the focused terminal's prompt as a paste (so an agent's input takes it like text you pasted).
//!
//! The card stays open while the pointer is on it or on the path, closes a moment after it has
//! left both, and closes on Esc. A click on the path itself opens the folder in Finder.

use std::path::Path;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::Button;
use wormhole_core::shells;

use super::title_row::TitlePlan;
use super::*;
use crate::style::motion::{PATH_CARD_CLOSE_DELAY, PATH_CARD_COPIED_FOR, PATH_CARD_OPEN_DELAY};

/// What the pointer is over: the folder in the title row, or the card under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Over {
    Path,
    Card,
}

#[derive(Default)]
pub(super) struct PathCard {
    shown: bool,
    over_path: bool,
    over_card: bool,
    /// Copy path was pressed: its label says "Copied" until the timer below clears it.
    copied: bool,
    /// The timer that opens the card, or closes it once the pointer has gone.
    timer: Option<Task<()>>,
    copied_timer: Option<Task<()>>,
}

impl PathCard {
    /// Whether the card is on screen (devctl's `title`, and the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn is_shown(&self) -> bool {
        self.shown
    }

    /// devctl: open or close the card at once, whatever the pointer does.
    #[cfg(feature = "devctl")]
    pub(super) fn devctl_set(&mut self, shown: bool) {
        self.reset();
        self.shown = shown;
        // Held open: the pointer isn't on it.
        self.over_card = shown;
    }

    /// Back to nothing: the window lost the front, or the terminal changed under the pointer.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The card's width at most: its three actions on one line need about this much.
const CARD_MAX_W: f32 = 440.0;
/// The gap between the title row and the card, and the card's distance from the column's edges.
const CARD_GAP: f32 = 4.0;
const CARD_MARGIN: f32 = 8.0;

impl Workspace {
    /// The pointer came onto or left the folder or the card: open after a moment on either, close a
    /// moment after both are left (so crossing the gap between them doesn't close it).
    pub(super) fn hover_path(&mut self, over: Over, hovered: bool, cx: &mut Context<Self>) {
        let card = &mut self.path_card;
        match over {
            Over::Path => card.over_path = hovered,
            Over::Card => card.over_card = hovered,
        }
        let want = card.over_path || card.over_card;
        if want == card.shown {
            // Nothing to change: a timer that was about to do the opposite is no longer wanted.
            card.timer = None;
            return;
        }
        let delay = if want { PATH_CARD_OPEN_DELAY } else { PATH_CARD_CLOSE_DELAY };
        card.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update(cx, |this, cx| {
                let card = &mut this.path_card;
                card.shown = card.over_path || card.over_card;
                cx.notify();
            })
            .ok();
        }));
    }

    /// Esc: close the card if it is open. Whether there was one.
    pub(super) fn close_path_card(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.path_card.shown {
            return false;
        }
        self.path_card.reset();
        cx.notify();
        true
    }

    /// The card under the folder, when it is open. `left` is where the folder starts in the
    /// terminal's column and `column` how wide that is: the card never reaches past either edge.
    pub(super) fn render_path_card(
        &self,
        plan: &TitlePlan,
        column: f32,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.path_card.shown {
            return None;
        }
        let (folder, shown) = (plan.facts.folder.clone()?, plan.facts.shown.clone()?);
        let ui = ui(cx);
        let copied = self.path_card.copied;
        let exited = plan.facts.status == Status::Exited;
        let width = (column - 2.0 * CARD_MARGIN).clamp(0.0, CARD_MAX_W);
        let left = plan.path_x.min(column - width - CARD_MARGIN).max(CARD_MARGIN);

        let segments = shown.split_inclusive('/').map(str::to_string).collect::<Vec<_>>();
        let last = segments.len().saturating_sub(1);
        let path = div().flex().flex_wrap().text_body().children(segments.into_iter().enumerate().map(
            |(ix, segment)| {
                let last = ix == last;
                div()
                    .flex_none()
                    .text_color(if last { ui.text } else { ui.muted })
                    .when(last, |d| d.font_weight(FontWeight::MEDIUM))
                    .child(segment)
            },
        ));
        let action = |id: &'static str, icon: IconName, label: &'static str| {
            Button::new(id).outline().small().icon(icon).label(label)
        };
        let (open_to, copy_of, insert_of) = (folder.clone(), folder.clone(), folder);
        let buttons = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(action("path-card-finder", IconName::FolderOpen, "Open in Finder").on_click(cx.listener(
                move |this, _, _, cx| {
                    cx.open_with_system(&open_to);
                    this.close_path_card(cx);
                },
            )))
            .child(
                action(
                    "path-card-copy",
                    if copied { IconName::Check } else { IconName::Copy },
                    if copied { "Copied" } else { "Copy path" },
                )
                .on_click(cx.listener(move |this, _, _, cx| this.copy_path(&copy_of, cx))),
            )
            .child(
                action("path-card-insert", IconName::SquareTerminal, "Insert into terminal")
                    .disabled(exited)
                    .when(exited, |b| b.tooltip("No terminal is running"))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.insert_path(&insert_of, window, cx)),
                    ),
            );
        Some(
            style::palette_surface(ui, true)
                .id("path-card")
                // Over the terminal: it takes the pointer, the clicks and the wheel from it.
                .occlude()
                .absolute()
                .top(px(style::TITLE_ROW_H + CARD_GAP))
                .left(px(left))
                .w(px(width))
                .gap_3()
                .p_3()
                .cursor(CursorStyle::Arrow)
                .role(Role::Group)
                .aria_label(format!("Folder {shown}"))
                .on_hover(
                    cx.listener(|this, hovered: &bool, _, cx| this.hover_path(Over::Card, *hovered, cx)),
                )
                .child(path)
                .child(buttons)
                .into_any_element(),
        )
    }

    /// Copy path: the plain absolute path, unquoted, on the clipboard. The card stays, and says so
    /// for a moment; VoiceOver says it too.
    fn copy_path(&mut self, folder: &Path, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(folder.to_string_lossy().into_owned()));
        crate::a11y::announce("Path copied", false);
        let card = &mut self.path_card;
        card.copied = true;
        card.copied_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PATH_CARD_COPIED_FOR).await;
            this.update(cx, |this, cx| {
                this.path_card.copied = false;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Insert into terminal: the folder's absolute path, in single quotes, typed at the focused
    /// terminal as a paste (bracketed when the program asked for it), without a Return. The card
    /// closes and the terminal has the keyboard.
    fn insert_path(&mut self, folder: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let text = insert_text(folder);
        let terminal = self.active_terminal();
        terminal.update(cx, |terminal, _| terminal.paste(&text));
        self.path_card.reset();
        self.focus_terminal(window, cx);
        cx.notify();
    }
}

/// What Insert into terminal types: the path in single quotes, so spaces and `$` survive.
pub(super) fn insert_text(folder: &Path) -> String {
    shells::single_quote(&folder.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::insert_text;

    #[test]
    fn insert_types_the_absolute_path_in_single_quotes() {
        assert_eq!(insert_text(Path::new("/Users/ben/Projects/tube")), "'/Users/ben/Projects/tube'");
        assert_eq!(insert_text(Path::new("/Users/ben/My Projects/$HOME")), "'/Users/ben/My Projects/$HOME'");
        assert_eq!(insert_text(Path::new("/Users/ben/it's")), r#"'/Users/ben/it'\''s'"#);
    }
}
