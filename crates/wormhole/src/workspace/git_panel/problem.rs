//! What the Changes and History tabs show when git cannot answer: git is missing (and how
//! to install it), git failed (its first line, Retry, the whole message to copy), or the
//! branch Changes compares with was deleted.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::{SettingsStore, UiColors, ui};
use crate::style::{self, ROW_H, TextRole};
use crate::theme_bridge::token;
use crate::workspace::Workspace;
use wormhole_core::git::GitError;

/// The git tab a problem is drawn in, which its wording and its Retry are for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GitTab {
    Changes,
    History,
}

/// What a problem says: a line for everyone, a hint, and something to read or run.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Wording {
    pub headline: &'static str,
    pub hint: Option<&'static str>,
    /// Shown in the code font: the command to run, or git's own first line.
    pub detail: String,
    /// The whole message, for Copy error. `None` when there is nothing more to copy.
    pub copy: Option<String>,
}

/// How a git problem reads in `tab`. `None` for a folder that is not in a repository,
/// which is the plain empty note.
pub(super) fn wording(tab: GitTab, error: &GitError) -> Option<Wording> {
    match error {
        GitError::NotARepo => None,
        GitError::GitMissing => Some(Wording {
            headline: match tab {
                GitTab::Changes => "Changes need git, and it isn't installed.",
                GitTab::History => "History needs git, and it isn't installed.",
            },
            hint: Some("Install the Xcode Command Line Tools, then retry."),
            detail: "xcode-select --install".into(),
            copy: None,
        }),
        failed => {
            let message = failed.to_string();
            Some(Wording {
                headline: "Couldn't read this repository.",
                hint: None,
                detail: message.lines().next().unwrap_or_default().to_string(),
                copy: Some(message),
            })
        }
    }
}

impl Workspace {
    /// In place of a git tab's list when git could not read the folder at all.
    pub(super) fn render_git_problem(
        &self,
        tab: GitTab,
        error: &GitError,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let Some(Wording { headline, hint, detail, copy }) = wording(tab, error) else {
            return style::empty_note(ui, "This terminal's folder isn't inside a git repository.")
                .px_4()
                .into_any_element();
        };
        let code_font = crate::fonts::code_font(cx).0;
        // Everything starts at the panel's 16 pt inset, like the rows it stands in for.
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(style::empty_note(ui, headline).px_4())
            .children(hint.map(|hint| div().px_4().text_caption().text_color(ui.muted).child(hint)))
            .child(
                div()
                    .mx_4()
                    .p_2()
                    .rounded(ui.radius)
                    // A well a step darker than the panel (lighter in a light theme), like a field.
                    .bg(token(&cx.global::<SettingsStore>().tokens, "input.background"))
                    .text_caption()
                    .text_color(ui.muted)
                    .font_family(code_font)
                    .font_features(crate::fonts::code_font_features(cx))
                    .child(detail),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_4()
                    .child(
                        button("git-retry", "Retry")
                            .on_click(cx.listener(move |this, _, _, cx| this.retry_git(tab, cx))),
                    )
                    .children(copy.map(|message| {
                        button("git-copy-error", "Copy error").on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(message.clone()))
                        })
                    })),
            )
            .into_any_element()
    }

    /// Ask git again, now: the Retry of a git problem.
    pub(super) fn retry_git(&mut self, tab: GitTab, cx: &mut Context<Self>) {
        let ix = self.active;
        match tab {
            GitTab::Changes => self.refresh_git(ix, cx),
            GitTab::History => self.refresh_history(ix, cx),
        }
    }
}

/// The red line over a list that is still the last good one: git failed this time, maybe
/// only for a moment (another process held the index), so the list is not thrown away.
pub(super) fn refresh_failed(ui: UiColors, error: &GitError) -> Div {
    let first = error.to_string().lines().next().unwrap_or_default().to_string();
    div()
        .px_4()
        .py_1()
        .text_caption()
        .text_color(ui.danger_text)
        .truncate()
        .child(format!("Couldn't refresh: {first}"))
}

/// Height of a text button in a panel: caption text like Kit's `xsmall`,
/// but 24 tall with the 8 inset of `small`. (`style/` has no panel text button yet.)
const BUTTON_H: f32 = 24.0;

/// An outlined text button: Retry, Copy error.
fn button(id: &'static str, label: &'static str) -> Button {
    Button::new(id).outline().xsmall().h(px(BUTTON_H)).px_2().label(label)
}

/// The amber line over the Changes list while the branch it compared with is gone: the list
/// is against HEAD until another branch is picked.
pub(super) fn compare_missing(ui: UiColors, branch: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(ROW_H))
        .px_4()
        .text_caption()
        .text_color(ui.warning_text)
        .child(style::lead(style::icon(IconName::Shield)))
        .child(div().min_w_0().truncate().child(format!("{branch} no longer exists · showing vs HEAD")))
}

#[cfg(test)]
mod tests {
    use super::{GitTab, Wording, wording};
    use wormhole_core::git::GitError;

    #[test]
    fn a_missing_git_says_how_to_install_it_and_names_the_tab() {
        let changes = wording(GitTab::Changes, &GitError::GitMissing).expect("a problem");
        assert_eq!(changes.headline, "Changes need git, and it isn't installed.");
        assert_eq!(changes.hint, Some("Install the Xcode Command Line Tools, then retry."));
        assert_eq!((changes.detail.as_str(), changes.copy), ("xcode-select --install", None));
        let history = wording(GitTab::History, &GitError::GitMissing).expect("a problem");
        assert_eq!(history.headline, "History needs git, and it isn't installed.");
    }

    #[test]
    fn a_failure_shows_gits_first_line_and_copies_all_of_it() {
        let message = "fatal: detected dubious ownership in repository at '/Volumes/work/app'\n\
                       To add an exception for this directory, call:";
        let failed = wording(GitTab::Changes, &GitError::Failed(message.into()));
        assert_eq!(
            failed,
            Some(Wording {
                headline: "Couldn't read this repository.",
                hint: None,
                detail: "fatal: detected dubious ownership in repository at '/Volumes/work/app'".into(),
                copy: Some(message.into()),
            })
        );
        let timed_out = wording(GitTab::History, &GitError::TimedOut(std::time::Duration::from_secs(30)));
        assert_eq!(timed_out.map(|w| w.detail), Some("git did not finish within 30s".into()));
    }

    #[test]
    fn not_a_repository_is_the_plain_note() {
        assert_eq!(wording(GitTab::Changes, &GitError::NotARepo), None);
    }
}
