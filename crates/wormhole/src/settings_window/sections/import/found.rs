//! The terminals found: a card each, newest settings first. A card's header names the terminal and
//! counts what it says; the open one lists the files read and what would be brought over (`review`).

use gpui_kit::assets::IconName;
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
        .gap_3()
        .children(all.iter().enumerate().map(|(ix, found)| card(window, found, ix, cx)))
        .into_any_element()
}

/// Nothing to bring over: what was looked for, so the page doesn't read as broken.
fn none_found(ui: crate::settings::UiColors) -> Div {
    style::card(ui, ui.border)
        .flex()
        .flex_col()
        .gap_2()
        .child(style::icon(IconName::Search).text_color(ui.muted))
        .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(text(SETTINGS_IMPORT_NONE)))
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_IMPORT_NONE_DETAIL)))
}

fn card(window: &SettingsWindow, found: &Found, ix: usize, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let source = found.source;
    let open = window.import.open == Some(source);
    let count = found.proposal.values.len();
    let name = terminal_name(source);
    let header = div()
        .id(("import-card", ix))
        .role(Role::Button)
        .aria_label(settings_import_card_label(name, &settings_import_count(whole(count))))
        .aria_expanded(open)
        // A tab stop, so ↩ and Space open and close it as a click does.
        .focusable()
        .tab_stop(true)
        .focus_visible(move |d| d.bg(ui.hover))
        .flex()
        .items_center()
        .gap_2()
        .min_h(px(style::CONTROL_H))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.import.open = if this.import.open == Some(source) { None } else { Some(source) };
            cx.notify();
        }))
        .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(name))
        .child(style::count_badge(ui, settings_import_count(whole(count))))
        .child(div().flex_1())
        .child(
            style::icon(if open { IconName::ChevronUp } else { IconName::ChevronDown }).text_color(ui.muted),
        );
    let mut card = style::card(ui, if open { ui.border_strong } else { ui.border })
        .flex()
        .flex_col()
        .gap_3()
        .child(header);
    if open {
        card = card.child(files_read(ui, found, cx)).child(window.render_review(found, cx));
    }
    card.into_any_element()
}

/// The files the settings were read from, so a person sees where they came from.
fn files_read(ui: crate::settings::UiColors, found: &Found, cx: &App) -> Div {
    let (code_font, _) = crate::fonts::code_font(cx);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(style::section_label(ui, text(SETTINGS_IMPORT_FILES_READ)))
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
