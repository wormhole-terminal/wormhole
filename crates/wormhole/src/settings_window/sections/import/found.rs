//! The terminals found: a row each, newest settings first. A row names the terminal, counts what it says and
//! shows the file it was read from; its button opens the files read and what would be brought over (`review`).

use gpui_kit::assets::IconName;
use gpui_kit::component::Selectable as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::import::{Found, visible};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use crate::settings::import::terminal_name;
use crate::settings_window::{SettingsWindow, ui, with_tilde};
use crate::style::{self, TextRole};

pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let page = &window.import;
    let Some(all) = page.found.as_ref() else {
        return style::empty_note(ui, text(SETTINGS_IMPORT_SCANNING)).into_any_element();
    };
    if all.is_empty() {
        return none_found(ui).into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .children(all.iter().enumerate().map(|(ix, found)| card(window, found, ix, cx)))
        .into_any_element()
}

/// Nothing to bring over: what was looked for, so the page doesn't read as broken.
fn none_found(ui: crate::settings::UiColors) -> Div {
    style::empty_list(
        ui,
        style::EmptyList {
            icon: IconName::Search,
            title: text(SETTINGS_IMPORT_NONE).into(),
            hint: text(SETTINGS_IMPORT_NONE_DETAIL).into(),
            actions: Vec::new(),
        },
    )
}

/// One terminal found: its name with how many settings it has, the file it was read from as a visible
/// hint (where the person's own settings are is what they check first), and the button that opens the list of
/// what it would bring over under the row.
fn card(window: &SettingsWindow, found: &Found, ix: usize, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let source = found.source;
    let open = window.import.open == Some(source);
    let name = terminal_name(source);
    let path = found.files.first().map(|path| visible(&with_tilde(path)).to_string());
    let button = style::form_button(("import-card", ix))
        .outline()
        .icon(IconName::ArrowDownToLine)
        .label(text(SETTINGS_IMPORT_REVIEW))
        .selected(open)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.import.open = if this.import.open == Some(source) { None } else { Some(source) };
            cx.notify();
        }))
        .into_any_element();
    let row = style::setting(
        SharedString::from(format!("import-row-{}", source.key())),
        ui,
        style::SettingParts {
            label: name.into(),
            description: path.map(Into::into),
            visible_hint: true,
            chip: Some(settings_import_count(whole(found.proposal.values.len())).into()),
            keys: style::RowKeys::NONE,
            state: style::SettingState::Default,
            found: None,
            found_description: None,
            control: button,
            tall: false,
            extra: None,
            on_reset: None,
        },
        cx,
    );
    let mut column = div().flex().flex_col().child(row);
    if open {
        column = column.child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .py_3()
                .child(files_read(ui, found, cx))
                .child(window.render_review(found, cx)),
        );
    }
    column.into_any_element()
}

/// The files the settings were read from, so a person sees where they came from.
fn files_read(ui: crate::settings::UiColors, found: &Found, cx: &App) -> Div {
    let (code_font, _) = crate::fonts::code_font(cx);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(style::setting_group_head(ui, text(SETTINGS_IMPORT_FILES_READ), None))
        .children(found.files.iter().map(|path| {
            div()
                .min_w_0()
                .truncate()
                .text_caption()
                .text_color(ui.muted)
                .font_family(code_font.clone())
                .child(visible(&with_tilde(path)))
        }))
}
