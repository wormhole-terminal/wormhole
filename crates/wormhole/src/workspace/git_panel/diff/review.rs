//! Comments on a diff's lines for the terminal's agent (core `review`): a click on a line opens a
//! small dialog to write one, or to change or remove the one it has; commented lines wear a bar
//! (`render`); and while the terminal has comments a bar under the rows gathers them, with Paste
//! into Terminal. That pastes the review into the terminal's prompt and gives it the keys; it never
//! presses Return, so the person reads what goes to the agent, and a plain shell runs nothing.

use std::rc::Rc;

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::dialog::{Cancel, Confirm, DialogFooter};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::review::{LineSide, ReviewComment};

use super::DiffView;
use super::render::{LineAt, OnLine};
use crate::settings::{UiColors, ui};
use crate::style::{self, Edges, TextRole};
use crate::workspace::{Workspace, dialog};

/// How tall the comment field grows, in lines.
const FIELD_ROWS: (usize, usize) = (3, 10);
/// The bar under the rows while there are comments.
const BAR_H: f32 = 40.0;

/// The comment dialog while it is up: which line, and its field.
pub(in crate::workspace) struct Commenting {
    path: String,
    at: LineAt,
    input: Entity<TextareaState>,
    /// ⌘↩ in the field saves.
    _enter: Subscription,
}

impl Workspace {
    /// What a click on a line of `view` does: comment on it.
    pub(super) fn comment_click(&self, view: &DiffView, cx: &mut Context<Self>) -> OnLine {
        let path = view.file().path.clone();
        let this = cx.weak_entity();
        Rc::new(move |at, window, cx| {
            let path = path.clone();
            let _ = this.update(cx, |ws, cx| ws.comment_on_line(path, at, window, cx));
        })
    }

    /// The lines of `view`'s file the active terminal has comments on.
    pub(super) fn commented_lines(&self, view: &DiffView) -> Vec<(LineSide, u32)> {
        self.active_session().review.lines_of(&view.file().path).collect()
    }

    /// The dialog for the comment on `at` in `path`: empty for a new one, with its text to change
    /// or remove an old one.
    fn comment_on_line(&mut self, path: String, at: LineAt, window: &mut Window, cx: &mut Context<Self>) {
        if self.commenting.is_some() {
            return;
        }
        let old = self.active_session().review.on(&path, at.side, at.line).map(|c| c.text.clone());
        let editing = old.is_some();
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(FIELD_ROWS.0, FIELD_ROWS.1)
                .placeholder("What should the agent change here?")
        });
        if let Some(old) = old {
            input.update(cx, |input, cx| input.set_value(old, window, cx));
        }
        let enter = cx.subscribe_in(&input, window, |_, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: true, .. }) {
                window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
            }
        });
        let place = ReviewComment {
            path: path.clone(),
            side: at.side,
            line: at.line,
            code: String::new(),
            text: String::new(),
        }
        .location();
        let code = SharedString::from(at.code.trim().to_string());
        self.commenting = Some(Commenting { path, at, input: input.clone(), _enter: enter });
        let weak = cx.weak_entity();
        let (font_family, _) = crate::fonts::code_font(cx);
        // As for the worktree dialog: nothing focused when it opens, so nothing is given the keys
        // back when it closes; `on_close` hands them to the terminal.
        window.blur(cx);
        dialog::open_alert(window, cx, move |alert, _, cx| {
            let ui = ui(cx);
            let (save, dismiss, remove) = (weak.clone(), weak.clone(), weak.clone());
            alert
                .title(format!("Comment on {place}"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .pt_2()
                        .when(!code.is_empty(), |d| {
                            d.child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(ui.radius_sm)
                                    .bg(ui.active)
                                    .font_family(font_family.clone())
                                    .text_caption()
                                    .text_color(ui.muted)
                                    .truncate()
                                    .child(code.clone()),
                            )
                        })
                        .child(Textarea::new(&input).small()),
                )
                .on_ok(move |_, _, cx| {
                    let _ = save.update(cx, |this, cx| this.save_comment(cx));
                    true
                })
                .on_close(move |_, window, cx| {
                    let _ = dismiss.update(cx, |this, cx| {
                        this.commenting = None;
                        cx.notify();
                        cx.defer_in(window, |this, window, cx| {
                            let terminal = this.active_terminal().focus_handle(cx);
                            window.focus(&terminal, cx);
                        });
                    });
                })
                .footer(
                    DialogFooter::new()
                        .child(style::key_hint(ui, "⌘↩", "saves"))
                        .child(div().flex_1())
                        .when(editing, |d| {
                            let remove = remove.clone();
                            d.child(Button::new("comment-remove").label("Remove").on_click(
                                move |_, window, cx| {
                                    let _ = remove.update(cx, |this, cx| this.remove_comment(cx));
                                    window.dispatch_action(Box::new(Cancel), cx);
                                },
                            ))
                        })
                        .child(Button::new("comment-cancel").label("Cancel").on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(Cancel), cx);
                        }))
                        .child(
                            Button::new("comment-save")
                                .label(if editing { "Save" } else { "Add Comment" })
                                .with_variant(ButtonVariant::Primary)
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
                                }),
                        ),
                )
        });
        cx.on_next_frame(window, |this, window, cx| {
            if let Some(commenting) = &this.commenting {
                let field = commenting.input.read(cx).focus_handle(cx);
                window.focus(&field, cx);
            }
        });
    }

    /// devctl `comment`: the open dialog's field set to `text`, then saved and closed.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_comment(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<String, &'static str> {
        let input = self.commenting.as_ref().ok_or("no comment dialog is open")?.input.clone();
        input.update(cx, |input, cx| input.set_value(text.to_string(), window, cx));
        // Saved now, so the summary counts it; the dialog's own save, as it closes, puts the same.
        self.save_comment(cx);
        window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
        Ok(self.active_session().review.summary())
    }

    /// The dialog's text becomes the line's comment; an empty one removes it.
    fn save_comment(&mut self, cx: &mut Context<Self>) {
        let Some(commenting) = &self.commenting else { return };
        let comment = ReviewComment {
            path: commenting.path.clone(),
            side: commenting.at.side,
            line: commenting.at.line,
            code: commenting.at.code.to_string(),
            text: commenting.input.read(cx).value().to_string(),
        };
        self.sessions[self.active].review.put(comment);
        cx.notify();
    }

    fn remove_comment(&mut self, cx: &mut Context<Self>) {
        let Some(commenting) = &self.commenting else { return };
        let empty = ReviewComment {
            path: commenting.path.clone(),
            side: commenting.at.side,
            line: commenting.at.line,
            code: String::new(),
            text: String::new(),
        };
        self.sessions[self.active].review.put(empty);
        cx.notify();
    }

    /// Paste the active terminal's review into its prompt and give it the keys: the person
    /// reads it there and presses Return. The comments are gone once pasted.
    pub(crate) fn paste_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(message) = self.active_session().review.message() else { return };
        self.active_terminal().update(cx, |terminal, _| terminal.paste(&message));
        self.sessions[self.active].review.clear();
        self.close_diff(window, cx);
    }

    pub(crate) fn clear_review(&mut self, cx: &mut Context<Self>) {
        self.sessions[self.active].review.clear();
        cx.notify();
    }

    /// The bar under the rows while the terminal has comments: how many, Clear, and Paste into
    /// Terminal.
    pub(super) fn render_review_bar(&self, ui: UiColors, cx: &mut Context<Self>) -> Option<AnyElement> {
        let review = &self.active_session().review;
        if review.is_empty() {
            return None;
        }
        Some(
            div()
                .id("diff-review")
                .flex()
                .flex_none()
                .items_center()
                .gap_2()
                .h(px(BAR_H))
                .pl_4()
                .pr_3()
                .rule_top(ui)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_body()
                        .text_color(ui.muted)
                        .child(review.summary()),
                )
                .child(
                    Button::new("review-clear")
                        .label("Clear")
                        .xsmall()
                        .with_variant(ButtonVariant::Ghost)
                        .on_click(cx.listener(|this, _, _, cx| this.clear_review(cx))),
                )
                .child(
                    Button::new("review-paste")
                        .label("Paste into Terminal")
                        .xsmall()
                        .with_variant(ButtonVariant::Primary)
                        .tooltip("Paste the comments into the terminal's prompt, for you to send")
                        .on_click(cx.listener(|this, _, window, cx| this.paste_review(window, cx))),
                )
                .into_any_element(),
        )
    }
}
