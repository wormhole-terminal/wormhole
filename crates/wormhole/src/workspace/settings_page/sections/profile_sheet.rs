//! The "New profile" sheet (RF-21): a name and what the profile starts from. It stands over the
//! settings window, opened from the Profile page, the File menu and the command palette. Create
//! makes the profile and switches Wormhole to it.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::config::profile::{NewProfile, StartFrom, first_letter};
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::storage::paths::ProfileId;

use crate::settings::SettingsStore;
use crate::style::{self, TextRole};
use crate::workspace::app::{AppState, app};
use crate::workspace::settings_page::{self, SettingsWindow, ui};

const SHEET_W: f32 = 380.0;

pub(in crate::workspace::settings_page) struct NewProfileSheet {
    name: Entity<InputState>,
    /// What it can start from, the profile Wormhole is in first chosen.
    options: Vec<(SharedString, StartFrom)>,
    chosen: usize,
    error: Option<SharedString>,
    _subscription: Subscription,
}

impl SettingsWindow {
    /// Open the sheet (a second call starts it afresh).
    pub(crate) fn open_new_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = app(cx);
        let shown = state.active_profile().clone();
        let mut options: Vec<(SharedString, StartFrom)> = state
            .profiles
            .profiles()
            .iter()
            .map(|p| (format!("A copy of {}", p.name).into(), StartFrom::Copy(p.id.clone())))
            .collect();
        options.push(("Wormhole's defaults".into(), StartFrom::Defaults));
        let chosen =
            options.iter().position(|(_, from)| *from == StartFrom::Copy(shown.clone())).unwrap_or(0);
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let subscription =
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.create_profile(window, cx),
                InputEvent::Change => {
                    if let Some(sheet) = this.new_profile.as_mut() {
                        sheet.error = None;
                    }
                    cx.notify();
                }
                _ => {}
            });
        window.focus(&name.focus_handle(cx), cx);
        self.new_profile =
            Some(NewProfileSheet { name, options, chosen, error: None, _subscription: subscription });
        cx.notify();
    }

    fn create_profile(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(sheet) = self.new_profile.as_mut() else { return };
        let name = sheet.name.read(cx).value().trim().to_string();
        if name.is_empty() {
            sheet.error = Some("Give the profile a name".into());
            cx.notify();
            return;
        }
        let from = sheet.options[sheet.chosen].1.clone();
        match AppState::create_profile(NewProfile { name, color: None, avatar: None, from }, cx) {
            Ok(id) => {
                self.new_profile = None;
                // Wormhole goes to the new profile (deferred: the windows are updating now).
                cx.defer(move |cx| switch_to(&id, cx));
            }
            Err(message) => sheet.error = Some(message.into()),
        }
        cx.notify();
    }

    /// The sheet over the window, with a scrim that keeps the page behind it still.
    pub(in crate::workspace::settings_page) fn render_new_profile(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let sheet = self.new_profile.as_ref()?;
        let ui = ui(cx);
        let state = app(cx);
        let tokens = &cx.global::<SettingsStore>().tokens;
        let color = crate::theme_bridge::hsla(state.profiles.next_color().color(tokens));
        let typed = sheet.name.read(cx).value();
        let letter = if typed.trim().is_empty() { '+' } else { first_letter(&typed) };
        let card = style::card(ui, ui.border_strong)
            .id("new-profile-sheet")
            .w(px(SHEET_W))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .bg(ui.elevated)
            .occlude()
            .child(style::panel_title(ui, "New profile"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(style::profile_avatar(ui, color, letter, PREVIEW_AVATAR))
                    .child(div().flex_1().child(Input::new(&sheet.name))),
            )
            .child(style::section_label(ui, "Start from"))
            .child(div().flex().flex_col().children(sheet.options.iter().enumerate().map(
                |(ix, (label, _))| {
                    style::choice_row(("new-profile-from", ix), ui, label.clone(), ix == sheet.chosen)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(sheet) = this.new_profile.as_mut() {
                                sheet.chosen = ix;
                            }
                            cx.notify();
                        }))
                },
            )))
            .child(
                div()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("Agent logins are never copied: the new profile signs in on its own."),
            )
            .children(
                sheet.error.clone().map(|error| div().text_caption().text_color(ui.danger_text).child(error)),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("new-profile-cancel").outline().small().label("Cancel").on_click(
                        cx.listener(|this, _, _, cx| {
                            this.new_profile = None;
                            cx.notify();
                        }),
                    ))
                    .child(
                        Button::new("new-profile-create")
                            .primary()
                            .small()
                            .label("Create")
                            .on_click(cx.listener(|this, _, window, cx| this.create_profile(window, cx))),
                    ),
            );
        Some(
            div()
                .id("new-profile-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(ui.shadow)
                .occlude()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.new_profile = None;
                    cx.notify();
                }))
                .child(card)
                .into_any_element(),
        )
    }
}

/// The avatar in front of the name field.
const PREVIEW_AVATAR: f32 = 24.0;

fn switch_to(id: &ProfileId, cx: &mut App) {
    if let Err(message) = AppState::switch_profile(id, cx) {
        tracing::warn!("switching to the new profile: {message}");
    }
}

/// The sheet over the settings window, from anywhere (the File menu, the palette, the page).
pub(crate) fn open_new_profile_sheet(cx: &mut App) {
    settings_page::open_settings(Some(Section::Profile), cx);
    // After `open_settings`'s own deferred opening, so the window exists by then.
    cx.defer(|cx| {
        let Some((handle, page)) = app(cx).settings_window.clone() else { return };
        let _ = handle.update(cx, |_, window, cx| {
            page.update(cx, |page, cx| page.open_new_profile(window, cx));
            crate::background::activate_window(window, cx);
        });
    });
}
