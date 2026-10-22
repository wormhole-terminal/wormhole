//! The profile chip beside the sidebar's toggle (and after the lights on the tab bar) and its menu
//! (RF-21). Wormhole is in one profile at a time and the chip names it; it is drawn only while there
//! are two or more profiles to choose between. Its menu switches Wormhole to a profile (every window
//! with it), opens a window in a profile or moves this one, and leads to the sheet that makes
//! a profile, to Settings and to the page that manages them. Each row runs the action the Shell →
//! Profile menu and the command palette run.

use gpui_kit::assets::IconName;
use gpui_kit::component::popover::Popover;
use gpui_kit::*;
use wormhole_core::keymap::{self, ActionId};
use wormhole_core::profile::ProfileId;
use wormhole_core::spoken;

use super::Workspace;
use super::actions::SwitchProfile;
use super::actions::{ManageProfiles, MoveWindowToProfile, NewProfile, OpenSettings, OpenWindowInProfile};
use super::app::app;
use crate::select;
use crate::settings::{UiColors, store_in, ui_in};
use crate::style;
use crate::style::{PopoverRow, popover_heading, popover_row, popover_rule};
use crate::theme_bridge;

/// Which list the open menu shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProfilePage {
    Main,
    /// "Open a window in…": the profiles to pick from.
    OpenIn,
    /// "Move this window to…".
    MoveTo,
}

/// The menu's width.
const MENU_W: f32 = 280.0;

/// A profile as the menu draws it.
#[derive(Clone)]
struct Choice {
    id: ProfileId,
    name: SharedString,
    letter: char,
    color: Hsla,
    /// Wormhole is in it now.
    current: bool,
    /// This window is in it.
    here: bool,
}

impl Workspace {
    /// Whether the chip is drawn: two or more profiles.
    pub(super) fn profile_chip_shown(&self, cx: &App) -> bool {
        app(cx).profiles.profiles().len() >= 2
    }

    /// The chip with its menu, or nothing while there is one profile.
    pub(super) fn render_profile_chip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.profile_chip_shown(cx) {
            return None;
        }
        let ui = ui_in(&self.profile, cx);
        let tokens = &store_in(&self.profile, cx).tokens;
        let state = app(cx);
        let active = state.profiles.active_id().clone();
        let choices: Vec<Choice> = state
            .profiles
            .profiles()
            .iter()
            .map(|profile| Choice {
                id: profile.id.clone(),
                name: profile.name.clone().into(),
                letter: profile.avatar,
                color: theme_bridge::hsla(profile.color.color(tokens)),
                current: profile.id == active,
                here: profile.id == self.profile,
            })
            .collect();
        let current = choices.iter().find(|choice| choice.current)?.clone();
        let page = self.sidebar.profile_menu;
        let settings_key = SharedString::from(keymap::glyphs(ActionId::Settings));
        let bg = select::menu_bg(ui, cx);
        let this = cx.weak_entity();
        let on_open = this.clone();
        let chip = style::profile_chip(
            "profile-chip",
            ui,
            current.color,
            current.letter,
            current.name.clone(),
            spoken::profile_chip(&current.name),
        );
        let popover = Popover::new("profile-menu")
            .anchor(Anchor::TopLeft)
            .offset(px(4.0))
            .open(page.is_some())
            .on_open_change(move |open, _, cx| {
                on_open
                    .update(cx, |ws, cx| {
                        ws.sidebar.profile_menu = open.then_some(ProfilePage::Main);
                        cx.notify();
                    })
                    .ok();
            })
            .trigger(chip);
        Some(
            select::menu_surface(popover, ui, px(MENU_W), bg)
                .p_1()
                .content(move |_, _, _| {
                    let Some(page) = page else { return div().into_any_element() };
                    profile_menu(page, &choices, &settings_key, ui, this.clone())
                })
                .into_any_element(),
        )
    }

    /// Close the menu.
    fn close_profile_menu(&mut self, cx: &mut Context<Self>) {
        self.sidebar.profile_menu = None;
        cx.notify();
    }
}

/// The menu's rows for `page`.
fn profile_menu(
    page: ProfilePage,
    choices: &[Choice],
    settings_key: &SharedString,
    ui: UiColors,
    this: WeakEntity<Workspace>,
) -> AnyElement {
    // A row that closes the menu and then runs `action`.
    let run = |action: Box<dyn Action>| {
        let this = this.clone();
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            this.update(cx, |ws, cx| ws.close_profile_menu(cx)).ok();
            window.dispatch_action(action.boxed_clone(), cx);
        }
    };
    // A row that stays open and shows `to`.
    let go = |to: ProfilePage| {
        let this = this.clone();
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            this.update(cx, |ws, cx| {
                ws.sidebar.profile_menu = Some(to);
                cx.notify();
            })
            .ok();
        }
    };
    let avatar = |choice: &Choice| {
        style::profile_avatar(ui, choice.color, choice.letter, style::TITLE_GLYPH).into_any_element()
    };
    let plain = |id: &'static str, label: &'static str| popover_row(id, ui, PopoverRow::new(label));
    let mut menu = div().flex().flex_col();
    match page {
        ProfilePage::Main => {
            menu = menu.child(popover_heading(ui, "Switch Wormhole to"));
            for choice in choices {
                let note = if choice.current { "current" } else { "every window switches" };
                let row = PopoverRow {
                    lead: Some(avatar(choice)),
                    label: choice.name.clone(),
                    note: Some(note.into()),
                    key: None,
                    current: choice.current,
                    spoken: spoken::profile_choice(&choice.name, choice.current).into(),
                };
                menu = menu.child(
                    popover_row(named("profile-switch", choice.id.as_str()), ui, row)
                        .on_click(run(Box::new(SwitchProfile(choice.id.as_str().to_string().into())))),
                );
            }
            menu = menu
                .child(popover_rule(ui))
                .child(plain("profile-open-in", "Open a window in…").on_click(go(ProfilePage::OpenIn)))
                .child(plain("profile-move-to", "Move this window to…").on_click(go(ProfilePage::MoveTo)))
                .child(popover_rule(ui))
                .child(plain("profile-new", "New profile…").on_click(run(Box::new(NewProfile))))
                .child(plain("profile-manage", "Manage profiles…").on_click(run(Box::new(ManageProfiles))))
                .child(popover_rule(ui))
                .child(
                    popover_row(
                        "profile-settings",
                        ui,
                        PopoverRow { key: Some(settings_key.clone()), ..PopoverRow::new("Settings…") },
                    )
                    .on_click(run(Box::new(OpenSettings))),
                );
        }
        ProfilePage::OpenIn | ProfilePage::MoveTo => {
            let opening = page == ProfilePage::OpenIn;
            menu = menu.child(
                popover_row(
                    "profile-back",
                    ui,
                    PopoverRow {
                        lead: Some(
                            style::icon(IconName::ChevronLeft).text_color(ui.muted).into_any_element(),
                        ),
                        spoken: "Back".into(),
                        ..PopoverRow::new(if opening { "Open a window in" } else { "Move this window to" })
                    },
                )
                .on_click(go(ProfilePage::Main)),
            );
            // A window already in a profile can't move to it.
            for choice in choices.iter().filter(|choice| opening || !choice.here) {
                let id = choice.id.as_str().to_string();
                let action: Box<dyn Action> = if opening {
                    Box::new(OpenWindowInProfile(id.clone().into()))
                } else {
                    Box::new(MoveWindowToProfile(id.clone().into()))
                };
                let row = PopoverRow {
                    lead: Some(avatar(choice)),
                    label: choice.name.clone(),
                    note: choice.here.then(|| "this window".into()),
                    ..PopoverRow::new(choice.name.clone())
                };
                menu = menu.child(popover_row(named("profile-pick", &id), ui, row).on_click(run(action)));
            }
        }
    }
    menu.into_any_element()
}

/// An element id made of a prefix and a profile's id.
fn named(prefix: &str, id: &str) -> ElementId {
    ElementId::Name(SharedString::from(format!("{prefix}-{id}")))
}
