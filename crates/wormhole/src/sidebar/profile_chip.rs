//! The profile chip and its platform menu: switch profiles, open or move a window,
//! create and manage profiles, and open Settings.

use gpui_kit::*;
use wormhole_core::config::profile::Profile;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::spoken;

use super::Sidebar;
use crate::actions::{
    ManageProfiles, MoveWindowToProfile, NewProfile, OpenSettings, OpenWindowInProfile, SwitchProfile,
};
use crate::menubar::model::KeyEquivalent;
use crate::platform::{self, Icon, Item, Menu};
use crate::settings::{UiColors, store_in, ui_in};
use crate::{style, theme_bridge};

/// The profiles and the one Wormhole is in.
#[derive(Debug)]
pub(crate) struct Profiles {
    pub(crate) list: Vec<Profile>,
    pub(crate) active: ProfileId,
}

pub(crate) type ProfilesFn = fn(&App) -> Option<Profiles>;

#[derive(Clone)]
struct Choice {
    id: ProfileId,
    name: SharedString,
    letter: char,
    color: Hsla,
    current: bool,
    here: bool,
}

impl Sidebar {
    /// The chip is present only when there are multiple profiles.
    pub(crate) fn render_profile_chip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Profiles { list, active } = (self.profiles)(cx)?;
        let ui = ui_in(&self.input.profile, cx);
        let tokens = &store_in(&self.input.profile, cx).tokens;
        let choices: Vec<Choice> = list
            .iter()
            .map(|profile| Choice {
                id: profile.id.clone(),
                name: profile.name.clone().into(),
                letter: profile.avatar,
                color: theme_bridge::hsla(profile.color.color(tokens)),
                current: profile.id == active,
                here: profile.id == self.input.profile,
            })
            .collect();
        let current = choices.iter().find(|choice| choice.current)?;
        let menu = profile_menu(&choices, ui);
        let anchor = platform::Anchor::default();
        let menu_anchor = anchor.clone();
        let chip = style::profile_chip(
            "profile-chip",
            ui,
            current.color,
            current.letter,
            current.name.clone(),
            spoken::profile_chip(&current.name),
        )
        .on_click(move |_, window, cx| {
            platform::show_menu(menu.clone(), menu_anchor.below(), window, cx);
        });
        Some(
            div()
                .relative()
                .debug_selector(|| "profile-chip".into())
                .child(anchor.probe())
                .child(chip)
                .into_any_element(),
        )
    }
}

/// A native menu and submenus retain the same profile actions and contextual notes.
fn profile_menu(choices: &[Choice], ui: UiColors) -> Menu {
    let icon = |choice: &Choice| {
        Some(Icon::Avatar {
            letter: choice.letter.to_uppercase().to_string().into(),
            background: choice.color,
            foreground: style::profile_avatar_ink(ui, choice.color),
        })
    };
    let action = |item: Item, action: Box<dyn Action>| {
        item.on_select(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    let mut menu = Menu::new().header("Switch Wormhole to");
    let mut open = Menu::new();
    let mut move_to = Menu::new();
    for choice in choices {
        let id = choice.id.as_str().to_string();
        let note = if choice.current { "current" } else { "every window switches" };
        let item = Item::new(choice.name.clone())
            .subtitle(Some(note.into()))
            .icon(icon(choice))
            .checked(choice.current);
        menu = menu.item(action(item, Box::new(SwitchProfile(id.clone().into()))));
        let item = Item::new(choice.name.clone())
            .icon(icon(choice))
            .subtitle(choice.here.then(|| "this window".into()));
        open = open.item(action(item, Box::new(OpenWindowInProfile(id.clone().into()))));
        if !choice.here {
            let item = Item::new(choice.name.clone()).icon(icon(choice));
            move_to = move_to.item(action(item, Box::new(MoveWindowToProfile(id.into()))));
        }
    }
    let settings_key = keymap::key(ActionId::Settings).and_then(KeyEquivalent::from_spec);
    menu.separator()
        .submenu("Open a window in", open)
        .submenu("Move this window to", move_to)
        .separator()
        .item(action(Item::new("New profile…"), Box::new(NewProfile)))
        .item(action(Item::new("Manage profiles…"), Box::new(ManageProfiles)))
        .separator()
        .item(action(Item::new("Settings…").key(settings_key), Box::new(OpenSettings)))
}
