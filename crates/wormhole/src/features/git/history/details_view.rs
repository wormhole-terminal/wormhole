//! The opened commit's details as the block under its row: the subject and the message (compact, with a
//! "Show more", or whole), the refs that point at it, who wrote and who committed it and when, its hash
//! and parents, and how much it changed. The block draws from the log's record at once and from what
//! [`Shown`](super::shown::Shown) made of the details once they are read; the list measures the row again
//! when they land. A frame builds no text of its own here and the block has a bounded number of
//! elements: what a long message or many refs would add is cut when the details are read.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::{Commit, RefKind};
use wormhole_core::view::spoken;
use wormhole_text::{keys, text};

use super::copy::CopyWhat;
use super::measure::caption_chars;
use super::menu::commit_menu;
use super::shown::{Fact, Shown, Tone};
use super::state::CommitInfo;
use super::view::{FILES_X, HistoryView};
use crate::platform;
use crate::settings::UiColors;
use crate::style::{self, Edges as _, TextRole};

/// The width of the labels ("Author", "Parents") the facts are lined up behind.
const LABEL_W: f32 = 72.0;

impl HistoryView {
    /// The block for the open commit; nothing when it is not in the log.
    pub(super) fn render_details(&self, ui: UiColors, window: &Window, cx: &App) -> AnyElement {
        let Some(commit) = &self.expanded_commit else { return div().into_any_element() };
        let shown = match &self.info {
            Some(CommitInfo::Loaded(shown)) => Some(shown),
            _ => None,
        };
        let spoken = match shown {
            Some(shown) => shown.spoken.clone(),
            None => spoken::commit_details(&commit.short, &commit.subject, &commit.author, "", 1).into(),
        };
        let (this, session) = (self.this.clone(), self.session);
        let mut block = div()
            .id("commit-details")
            .role(Role::Group)
            .aria_label(spoken)
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .pl(px(FILES_X))
            .pr(px(style::CARD_INSET))
            .py_2()
            .text_caption()
            // The menu a key reaches with ↩ on the row: the copies, "Show more", the parents.
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                cx.stop_propagation();
                this.update(cx, |this, cx| this.show_details_menu(session, event.position, window, cx)).ok();
            })
            .child(self.heading(commit, shown, ui));
        block = block.children(self.message(shown, ui, window, cx));
        if let Some(shown) = shown {
            block = block.children(refs(shown, ui));
        }
        block.child(self.facts(commit, shown, ui)).into_any_element()
    }

    /// The subject, in full, with the button that copies the whole message.
    fn heading(&self, commit: &Commit, shown: Option<&std::rc::Rc<Shown>>, ui: UiColors) -> Div {
        let subject: SharedString = match shown {
            Some(shown) => shown.subject.clone(),
            None => commit.subject.clone().into(),
        };
        div()
            .flex()
            .items_start()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_body()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ui.text)
                    .child(subject),
            )
            .child(self.copy_button(
                "history-copy-message",
                text(keys::HISTORY_COPY_MESSAGE),
                CopyWhat::Message,
                &commit.sha,
                ui,
            ))
    }

    /// The message: compact (about three lines of its first paragraph, and a "Show more" when something is
    /// left out) or whole (in at most two text elements: the prose and the trailers). What is not there yet,
    /// or could not be read, is a quiet line.
    fn message(
        &self,
        shown: Option<&std::rc::Rc<Shown>>,
        ui: UiColors,
        window: &Window,
        cx: &App,
    ) -> Vec<AnyElement> {
        let line = |color: Hsla, text: SharedString| div().text_color(color).child(text);
        let Some(shown) = shown else {
            return vec![
                match &self.info {
                    Some(CommitInfo::Failed(err)) => line(ui.danger_text, err.clone()),
                    _ => line(ui.muted, text(keys::HISTORY_READING_DETAILS).into()),
                }
                .into_any_element(),
            ];
        };
        let room = self.width - 2.0 * style::PANEL_ROW_X - style::INDENT;
        let (compact, more) = shown.compact(caption_chars(room, window, cx));
        let mut out: Vec<AnyElement> = Vec::new();
        if self.body_expanded {
            if !shown.whole.is_empty() {
                out.push(line(ui.text, shown.whole.clone()).into_any_element());
            }
            if !shown.trailers.is_empty() {
                out.push(line(ui.muted, shown.trailers.clone()).into_any_element());
            }
            if shown.body_cut {
                out.push(line(ui.muted, text(keys::HISTORY_MESSAGE_CONTINUES).into()).into_any_element());
            }
            if let Some(note) = &shown.message_cut {
                out.push(line(ui.muted, note.clone()).into_any_element());
            }
        } else if !compact.is_empty() {
            out.push(line(ui.text, compact).into_any_element());
        }
        if more {
            let (this, session) = (self.this.clone(), self.session);
            let key = if self.body_expanded { keys::HISTORY_SHOW_LESS } else { keys::HISTORY_SHOW_MORE };
            out.push(
                style::more_toggle("history-more", ui, text(key), self.body_expanded)
                    .on_click(move |_, _, cx| {
                        this.update(cx, |this, cx| this.toggle_body(session, cx)).ok();
                    })
                    .into_any_element(),
            );
        }
        out
    }

    /// Author, committer (when it differs) and co-authors, the hash with its buttons, the parents and the
    /// size of the change: a labelled line each, under a rule.
    fn facts(&self, commit: &Commit, shown: Option<&std::rc::Rc<Shown>>, ui: UiColors) -> Div {
        let mut rows: Vec<AnyElement> = Vec::new();
        match shown {
            Some(shown) => rows.extend(shown.people.iter().map(|fact| text_fact(ui, fact))),
            None => rows.push(
                fact_row(
                    ui,
                    text(keys::HISTORY_AUTHOR).into(),
                    div().child(SharedString::from(commit.author.clone())),
                    None,
                )
                .into_any_element(),
            ),
        }
        rows.push(self.hash_row(commit, ui));
        if let Some(shown) = shown
            && let Some(row) = self.parents_row(shown, ui)
        {
            rows.push(row);
        }
        if let Some(stat) = &self.stat {
            rows.push(text_fact(ui, stat));
        }
        div().flex().flex_col().gap_1().pt_2().rule_top(ui).children(rows)
    }

    /// The short hash, the button that copies the full one, and the menu of every other copy: the author,
    /// the subject and the short hash have no button of their own, which every frame would pay for.
    fn hash_row(&self, commit: &Commit, ui: UiColors) -> AnyElement {
        let code = div().font_family(self.code_font.clone()).child(commit.short.clone());
        let (this, sha, session) = (self.this.clone(), commit.sha.clone(), self.session);
        let more =
            style::icon_button("history-copy-more", ui, IconName::Ellipsis, text(keys::HISTORY_MORE_COPY))
                .in_row()
                .on_click(move |_, window, cx| {
                    platform::show_menu(
                        commit_menu(&this, session, &sha),
                        window.mouse_position(),
                        window,
                        cx,
                    );
                });
        let buttons = div()
            .flex()
            .items_center()
            .child(self.copy_button(
                "history-copy-hash",
                text(keys::HISTORY_COPY_HASH),
                CopyWhat::Hash,
                &commit.sha,
                ui,
            ))
            .child(more)
            .into_any_element();
        fact_row(ui, text(keys::HISTORY_COMMIT).into(), code, Some(buttons)).into_any_element()
    }

    /// The parents as links (the ones the log lists open when clicked), at most a few, the rest counted.
    fn parents_row(&self, shown: &Shown, ui: UiColors) -> Option<AnyElement> {
        if shown.parents.is_empty() {
            return None;
        }
        let links = shown.parents.iter().enumerate().map(|(ix, parent)| {
            // Only a parent the log lists can be opened from here.
            if !self.commits.iter().any(|c| c.sha == parent.sha) {
                return div()
                    .font_family(self.code_font.clone())
                    .child(parent.short.clone())
                    .into_any_element();
            }
            let (this, sha, session) = (self.this.clone(), parent.sha.clone(), self.session);
            style::hash_link(("history-parent", ix), ui, parent.short.clone(), parent.spoken.clone())
                .font_family(self.code_font.clone())
                .on_click(move |_, _, cx| {
                    this.update(cx, |this, cx| this.reveal_parent(session, sha.clone(), cx)).ok();
                })
                .into_any_element()
        });
        let row = div()
            .flex()
            .flex_wrap()
            .gap_x_2()
            .children(links)
            .children(shown.parents_more.clone().map(|more| div().text_color(ui.muted).child(more)));
        Some(fact_row(ui, shown.parents_label.clone(), row, None).into_any_element())
    }

    /// An icon button that copies `what` of the commit `sha`.
    fn copy_button(
        &self,
        id: &'static str,
        tooltip: &'static str,
        what: CopyWhat,
        sha: &str,
        ui: UiColors,
    ) -> AnyElement {
        let (this, sha, session) = (self.this.clone(), sha.to_string(), self.session);
        style::icon_button(id, ui, IconName::Copy, tooltip)
            .in_row()
            .on_click(move |_, window, cx| {
                let sha = sha.clone();
                this.update(cx, |this, cx| this.copy_commit(session, &sha, what, window, cx)).ok();
            })
            .into_any_element()
    }
}

/// A labelled fact: the label in a column of its own, the value after it, a button at the end. Three
/// elements.
fn fact_row(ui: UiColors, label: SharedString, value: impl IntoElement, trailing: Option<AnyElement>) -> Div {
    div()
        .flex()
        .items_start()
        .gap_2()
        .child(div().flex_none().w(px(LABEL_W)).text_color(ui.muted).child(label))
        .child(div().flex_1().min_w_0().text_color(ui.text).child(value))
        .children(trailing)
}

/// A fact whose text is in one piece, with stretches in other colors (an address, a date, `+4`).
fn text_fact(ui: UiColors, fact: &Fact) -> AnyElement {
    let tint = |tone: &Tone| HighlightStyle {
        color: Some(match tone {
            Tone::Muted => ui.muted,
            Tone::Added => ui.added,
            Tone::Removed => ui.removed,
        }),
        ..HighlightStyle::default()
    };
    let styled = StyledText::new(fact.text.clone())
        .with_highlights(fact.spans.iter().map(|(range, tone)| (range.clone(), tint(tone))));
    fact_row(ui, fact.label.clone(), styled, None).into_any_element()
}

/// The branches, tags and HEAD that point at the commit, as chips (a few, the rest counted); nothing
/// when none does.
fn refs(shown: &Shown, ui: UiColors) -> Option<AnyElement> {
    if shown.refs.is_empty() {
        return None;
    }
    let chips = shown.refs.iter().map(|(kind, name)| match kind {
        RefKind::Head => style::status_badge(ui, ui.primary, "HEAD").into_any_element(),
        RefKind::Tag => style::status_badge(ui, ui.tag, name.clone()).into_any_element(),
        RefKind::Branch | RefKind::Remote => style::branch_chip(ui, Some(name), None).into_any_element(),
    });
    let more = shown.refs_more.clone().map(|more| div().text_color(ui.muted).child(more));
    Some(div().flex().flex_wrap().items_center().gap_1().children(chips).children(more).into_any_element())
}
