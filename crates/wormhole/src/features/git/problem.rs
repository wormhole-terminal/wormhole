//! What the Changes and History tabs show when git cannot answer: git is missing (and how
//! to install it), git failed (its first line, Retry, the whole message to copy), or the
//! branch Changes compares with was deleted.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::settings::{store_in, ui_in};
use crate::style::{self, ROW_H, TextRole};
use crate::theme_bridge::token;
use wormhole_core::git::GitError;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::notice::Copied;

pub(super) use wormhole_core::git::problem::{GitTab, Wording, wording};

/// In place of a git tab's list when git could not read the folder at all. `retry` asks git again.
pub(super) fn render_git_problem(
    profile: &ProfileId,
    tab: GitTab,
    error: &GitError,
    retry: impl Fn(&mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let ui = ui_in(profile, cx);
    let Some(Wording { headline, hint, detail, copy }) = wording(tab, error) else {
        return style::empty_note(ui, "This terminal's folder isn't inside a git repository.")
            .into_any_element();
    };
    let code_font = crate::fonts::code_font_in(profile, cx).0;
    // Everything starts at the panel's 16 pt inset, like the rows it stands in for.
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(style::empty_note(ui, headline))
        .children(hint.map(|hint| div().px_3().text_caption().text_color(ui.muted).child(hint)))
        .child(
            div()
                .mx_3()
                .p_2()
                .rounded(ui.radius)
                // A well a step darker than the panel (lighter in a light theme), like a field.
                .bg(token(&store_in(profile, cx).tokens, "input.background"))
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
                .px_3()
                .pb_2()
                .child(button("git-retry", "Retry").on_click(move |_, _, cx| retry(cx)))
                .children(copy.map(|message| {
                    button("git-copy-error", "Copy error").on_click(move |_, window, cx| {
                        crate::platform::clipboard::write(message.clone(), cx);
                        crate::notice::post(window, Copied::Error, None, cx);
                    })
                })),
        )
        .into_any_element()
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
