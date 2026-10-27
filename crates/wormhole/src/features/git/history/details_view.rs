//! The opened commit's details as the block under its row: the subject and the whole message, the refs
//! that point at it, who wrote and who committed it and when, its hash and parents, and how much it
//! changed. The block draws from the log's record at once and from the commit's details once they are
//! read; the list measures the row again when they land.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::{Block, Commit, CommitDetails, CommitRef, DiffStat, RefKind, Signature};
use wormhole_core::view::panel::age;
use wormhole_core::view::spoken;

use super::copy::CopyWhat;
use super::menu::commit_menu;
use super::state::CommitInfo;
use super::view::{FILES_X, HistoryView};
use crate::platform;
use crate::settings::UiColors;
use crate::style::{self, Edges as _, TextRole};

/// The width of the labels ("Author", "Parents") the facts are lined up behind.
const LABEL_W: f32 = 72.0;

impl HistoryView {
    /// The block for the open commit; nothing when it is not in the log.
    pub(super) fn render_details(&self, ui: UiColors) -> AnyElement {
        let Some(commit) = &self.expanded_commit else { return div().into_any_element() };
        let details = match &self.info {
            Some(CommitInfo::Loaded(details)) => Some(details),
            _ => None,
        };
        let subject = details.map_or(commit.subject.as_str(), |d| d.subject()).to_string();
        let when = details.map(|d| d.author.when()).unwrap_or_default();
        let parents = details.map_or(1, |d| d.parents.len());
        let spoken = spoken::commit_details(&commit.short, &subject, &commit.author, &when, parents);

        div()
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
            .child(self.heading(commit, subject, ui))
            .children(self.message(details, ui))
            .children(refs(details.map_or(&[][..], |d| &d.refs), ui))
            .child(self.facts(commit, details, ui))
            .into_any_element()
    }

    /// The subject, in full, with the button that copies the whole message.
    fn heading(&self, commit: &Commit, subject: String, ui: UiColors) -> Div {
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
                "Copy message",
                CopyWhat::Message,
                &commit.sha,
                ui,
            ))
    }

    /// The body, block by block; what is not there yet, or could not be read, in a quiet line.
    fn message(&self, details: Option<&std::rc::Rc<CommitDetails>>, ui: UiColors) -> Vec<AnyElement> {
        let line = |color: Hsla, text: SharedString| div().text_color(color).child(text).into_any_element();
        match (&self.info, details) {
            (_, Some(details)) => {
                let mut blocks: Vec<AnyElement> = details
                    .blocks()
                    .into_iter()
                    .map(|block| match block {
                        Block::Paragraph(text) | Block::Lines(text) => line(ui.text, text.into()),
                        Block::Trailers(text) => line(ui.muted, text.into()),
                    })
                    .collect();
                if details.message_cut {
                    blocks.push(line(
                        ui.muted,
                        "The message is longer than this; the rest is left out.".into(),
                    ));
                }
                blocks
            }
            (Some(CommitInfo::Failed(err)), _) => vec![line(ui.danger_text, err.clone())],
            _ => vec![line(ui.muted, "Reading details…".into())],
        }
    }

    /// Author, committer (when it is someone else, or another time), co-authors, the hash with its
    /// buttons, the parents and the size of the change.
    fn facts(&self, commit: &Commit, details: Option<&std::rc::Rc<CommitDetails>>, ui: UiColors) -> Div {
        let now = self.now;
        let mut rows: Vec<AnyElement> = Vec::new();
        match details {
            Some(details) => {
                rows.push(person_row(ui, "Author", &details.author, now));
                if details.committer_differs() {
                    rows.push(person_row(ui, "Committer", &details.committer, now));
                }
                for co in &details.co_authors {
                    rows.push(fact(ui, "Co-author", named(ui, &co.name, &co.email), None));
                }
            }
            None => rows.push(fact(
                ui,
                "Author",
                named(ui, &commit.author, &format!("{} ago", age(now - commit.time))),
                None,
            )),
        }
        rows.push(self.hash_row(commit, ui));
        if let Some(details) = details
            && let Some(row) = self.parents_row(details, ui)
        {
            rows.push(row);
        }
        if let Some(files) = &self.files {
            rows.push(stat_row(DiffStat::of(files), ui));
        }
        div().flex().flex_col().gap_1().pt_2().rule_top(ui).children(rows)
    }

    /// The short hash (the full one behind its button), and the menu of every other copy: the author, the
    /// subject and the short hash have no button of their own, which every frame would pay for.
    fn hash_row(&self, commit: &Commit, ui: UiColors) -> AnyElement {
        let code = div().font_family(self.code_font.clone()).text_color(ui.text).child(commit.short.clone());
        let (this, sha, session) = (self.this.clone(), commit.sha.clone(), self.session);
        let more = style::icon_button("history-copy-more", ui, IconName::Ellipsis, "More ways to copy")
            .in_row()
            .on_click(move |_, window, cx| {
                platform::show_menu(commit_menu(&this, session, &sha), window.mouse_position(), window, cx);
            });
        let buttons = div()
            .flex()
            .items_center()
            .child(self.copy_button("history-copy-hash", "Copy hash", CopyWhat::Hash, &commit.sha, ui))
            .child(more);
        fact(ui, "Commit", code, Some(buttons.into_any_element()))
    }

    fn parents_row(&self, details: &CommitDetails, ui: UiColors) -> Option<AnyElement> {
        if details.parents.is_empty() {
            return None;
        }
        let label = if details.parents.len() > 1 { "Parents" } else { "Parent" };
        let parents = details.parents.iter().fold(div().flex().flex_wrap().gap_x_2(), |row, parent| {
            let short: String = parent.chars().take(7).collect();
            // Only a parent the log lists can be opened from here.
            let listed = self.commits.iter().any(|c| &c.sha == parent);
            let (this, sha, session) = (self.this.clone(), parent.clone(), self.session);
            row.child(
                div()
                    .id(SharedString::from(format!("history-parent-{parent}")))
                    .font_family(self.code_font.clone())
                    .when(!listed, |d| d.text_color(ui.text))
                    .when(listed, |d| {
                        d.text_color(ui.link).cursor_pointer().hover(|d| d.underline()).on_click(
                            move |_, _, cx| {
                                this.update(cx, |this, cx| this.reveal_parent(session, sha.clone(), cx)).ok();
                            },
                        )
                    })
                    .child(short),
            )
        });
        Some(fact(ui, label, parents, None))
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

/// A labelled fact: the label in a column of its own, the value after it, a button at the end.
fn fact(
    ui: UiColors,
    label: &'static str,
    value: impl IntoElement,
    trailing: Option<AnyElement>,
) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap_2()
        .child(div().flex_none().w(px(LABEL_W)).text_color(ui.muted).child(label))
        .child(div().flex_1().min_w_0().text_color(ui.text).child(value))
        .children(trailing)
        .into_any_element()
}

/// Someone's name and address, and under them when: "27 Oct 2026, 14:05 +02:00 · 3d ago".
fn person_row(ui: UiColors, label: &'static str, who: &Signature, now: i64) -> AnyElement {
    let ago = match age(now - who.time).as_str() {
        "now" => "just now".to_string(),
        // No break before "ago": it would stand alone on a line of its own.
        ago => format!("{ago}\u{a0}ago"),
    };
    let when = who.when();
    let value = div().flex().flex_col().child(named(ui, &who.name, &who.email)).child(
        div().text_color(ui.muted).child(if when.is_empty() { ago } else { format!("{when} · {ago}") }),
    );
    fact(ui, label, value, None)
}

/// A name and, after it in the muted color, the address (or whatever else goes with it).
fn named(ui: UiColors, name: &str, rest: &str) -> Div {
    div()
        .flex()
        .flex_wrap()
        .gap_x_1()
        .child(name.to_string())
        .child(div().text_color(ui.muted).child(rest.to_string()))
}

/// "3 files  +12 −4", the numbers in the colors of the file rows.
fn stat_row(stat: DiffStat, ui: UiColors) -> AnyElement {
    let files = if stat.files == 1 { "1 file".to_string() } else { format!("{} files", stat.files) };
    let value = div()
        .flex()
        .gap_2()
        .tabular_nums()
        .child(files)
        .child(div().text_color(ui.added).child(format!("+{}", stat.added)))
        .child(div().text_color(ui.removed).child(format!("\u{2212}{}", stat.removed)));
    fact(ui, "Changes", value, None)
}

/// The branches, tags and HEAD that point at the commit, as chips; nothing when none does.
fn refs(refs: &[CommitRef], ui: UiColors) -> Option<AnyElement> {
    if refs.is_empty() {
        return None;
    }
    let chips = refs.iter().map(|r| match r.kind {
        RefKind::Head => style::status_badge(ui, ui.primary, "HEAD").into_any_element(),
        RefKind::Tag => style::status_badge(ui, ui.warning_text, r.name.clone()).into_any_element(),
        RefKind::Branch | RefKind::Remote => {
            style::branch_chip(ui, Some(r.name.as_str()), None).into_any_element()
        }
    });
    Some(div().flex().flex_wrap().items_center().gap_1().children(chips).into_any_element())
}
