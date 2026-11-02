//! Menu bar & hotkey's preview: a stretch of the menu bar with Wormhole's extra (the dot and the count
//! while a terminal needs input) and the menu it opens, compiled from the design's `MenuBarMock`
//! (`ui/generated/menu_bar_mock.rs`). This file fills it from the page's switch and icon choice.

use gpui_kit::*;
use wormhole_core::config::MenuBarIcon;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::card::{Words, card};
use crate::settings::{SettingsStore, settings};
use crate::settings_window::{SettingsWindow, ui};
use crate::theme_bridge::token;
use crate::ui::{Look, MenuBarItem, MenuBarMockProps};

/// The menu bar, the extra and its menu.
pub(in crate::settings_window) fn draw_menu_bar_preview(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx);
    let (shown, icon) = (s.menu_bar, s.menu_bar_icon);
    let tokens = &cx.global::<SettingsStore>().tokens;
    let (on_primary, muted) = (token(tokens, "text.on_primary"), token(tokens, "text.muted"));
    // text: sample (the made-up terminals and clock of the preview)
    let items = [("Flaky login test", "⌘1", true), ("Rate limits", "⌘2", false)]
        .into_iter()
        .map(|(name, key, selected)| MenuBarItem {
            name: name.into(),
            key: key.into(),
            key_color: if selected { on_primary } else { muted },
            selected,
        })
        .collect();
    let mock = MenuBarMockProps {
        id: "menu-bar-mock".into(),
        spoken: text(SETTINGS_WINDOW_PREVIEW_MENU_BAR_LABEL).into(),
        shown,
        dot: icon != MenuBarIcon::Plain,
        count: icon == MenuBarIcon::StatusCount,
        count_text: "2".into(),    // text: sample (a made-up count)
        clock: "Thu 09:41".into(), // text: sample (a made-up clock)
        header: text(SETTINGS_MENU_BAR_PREVIEW_WAITING).into(),
        items,
    };
    card(
        ui,
        Words {
            id: "menu-bar-preview-card",
            what: SETTINGS_WINDOW_MENU_BAR,
            spoken: SETTINGS_WINDOW_PREVIEW_MENU_BAR_LABEL,
            caption: SETTINGS_MENU_BAR_PREVIEW_CAPTION,
        },
        div().child(mock.render(&Look::of(cx))).overflow_hidden(),
    )
}
