//! A group's popover and context menu, from one description of the group (`GroupFacts`). The name
//! on its head opens the popover: the name in a field that has the keyboard at once, the nine
//! colours (the current one ringed) with an empty ring first that takes the colour away and a last
//! dot that opens the system's Colours window for a colour of one's own, then the group's folder
//! (shown greyed out, with Set folder… and, when there is one, Clear folder) and the actions. The
//! right-click menu is Rename… (in place, `rename`), the same dots, Other colour…, No colour
//! (checked when the group has none), the same folder lines, and the same actions.

use gpui_kit::*;
use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::sidebar::{GroupColor, GroupPaint};

use super::groups::group_colors;
use super::{Sidebar, SidebarEvent};
use crate::platform::popover::{Behavior, Colours, Custom, Field, Popover, Row};
use crate::platform::{self, Item, Menu, Swatch};
use crate::settings::store_in;

/// What a group's popover and menu are made from, as it is when they open.
pub(super) struct GroupFacts {
    pub(super) group: u32,
    /// What it is painted with, if anything.
    pub(super) paint: Option<GroupPaint>,
    /// Its colour of its own as it was chosen (the dot shows that, not what the theme made of it).
    pub(super) custom: Option<Hsla>,
    /// What it wears now, where the Colours window starts.
    pub(super) shown: Option<Hsla>,
    pub(super) pinned: bool,
    /// The group's folder as the list shows it (`~/Projects/app`), if it has one.
    pub(super) folder: Option<String>,
    /// Every named colour on offer, with how the theme draws it.
    pub(super) colors: Vec<(GroupColor, Hsla)>,
}

impl Sidebar {
    pub(super) fn group_facts(&self, group: u32, cx: &App) -> GroupFacts {
        let found = self.view.groups.get(group);
        let paint = found.and_then(|g| g.paint());
        let tokens = &store_in(&self.input.profile, cx).tokens;
        GroupFacts {
            group,
            paint,
            custom: match paint {
                Some(GroupPaint::Custom([r, g, b])) => Some(from_bytes([r, g, b])),
                _ => None,
            },
            shown: paint.map(|paint| crate::theme_bridge::hsla(paint.color(tokens))),
            pinned: found.is_some_and(|g| g.pinned),
            folder: self.group_folder_label(group),
            colors: group_colors(&self.input.profile, cx),
        }
    }
}

fn from_bytes([r, g, b]: [u8; 3]) -> Hsla {
    Rgba { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a: 1.0 }.into()
}

/// What the empty ring at the start of the popover's dots is called, and the menu's item.
const NO_COLOUR: &str = "No colour";

/// The named colours as dots, which is ringed, and what a click on one does. With `none` an empty
/// ring comes first, which takes the colour away and is ringed while the group has none (the
/// menu has an item for that instead).
fn dots(
    facts: &GroupFacts,
    none: bool,
    this: &WeakEntity<Sidebar>,
) -> (Vec<Swatch>, Option<usize>, platform::menu::Pick) {
    let lead = usize::from(none);
    let swatches: Vec<Swatch> = none
        .then(|| Swatch::none(NO_COLOUR))
        .into_iter()
        .chain(facts.colors.iter().map(|&(color, hsla)| Swatch { color: hsla, name: color.label().into() }))
        .collect();
    let chosen = if facts.paint.is_none() {
        none.then_some(0)
    } else {
        facts
            .colors
            .iter()
            .position(|&(color, _)| facts.paint == Some(GroupPaint::Named(color)))
            .map(|ix| ix + lead)
    };
    let (group, colors, this) = (facts.group, facts.colors.clone(), this.clone());
    let on_pick: platform::menu::Pick = std::rc::Rc::new(move |ix, _, cx| match ix.checked_sub(lead) {
        Some(ix) => {
            if let Some(&(color, _)) = colors.get(ix) {
                this.update(cx, |sidebar, cx| sidebar.set_group_color(group, Some(color), cx)).ok();
            }
        }
        None => {
            this.update(cx, |sidebar, cx| sidebar.set_group_color(group, None, cx)).ok();
        }
    });
    (swatches, chosen, on_pick)
}

/// What the Colours window does with each colour it sends: the group follows it.
fn follow(group: u32, this: &WeakEntity<Sidebar>) -> platform::ColourChange {
    let this = this.clone();
    std::rc::Rc::new(move |color, _, cx| {
        this.update(cx, |sidebar, cx| sidebar.set_group_custom_color(group, color, cx)).ok();
    })
}

/// The grey the Colours window starts at for a group with no colour.
const START: Hsla = Hsla { h: 0.0, s: 0.0, l: 0.5, a: 1.0 };

/// The group's folder, which both the popover and the menu list above the actions: an info line
/// (greyed out, it is not a choice) when there is one, then what changes it.
fn folder_items(group: u32, folder: Option<&str>, this: &WeakEntity<Sidebar>) -> Vec<Item> {
    let set = Item::new("Set folder…")
        .on_select(platform::run_on(this, move |sidebar, _, cx| sidebar.choose_group_folder(group, cx)));
    match folder {
        Some(folder) => vec![
            Item::new(folder).enabled(false),
            set,
            Item::new("Clear folder").on_select(platform::run_on(this, move |sidebar, _, cx| {
                sidebar.set_group_folder(group, None, cx)
            })),
        ],
        None => vec![set],
    }
}

/// The actions both the popover and the menu end with, in their order.
fn actions(group: u32, pinned: bool, this: &WeakEntity<Sidebar>) -> Vec<Item> {
    vec![
        Item::new("New terminal in group").on_select(platform::run_on(this, move |_, _, cx| {
            cx.emit(SidebarEvent::NewTerminal(super::NewTerminal::InGroup(group)))
        })),
        Item::new("Ungroup")
            .on_select(platform::run_on(this, move |sidebar, _, cx| sidebar.ungroup(group, cx))),
        Item::new("Close group")
            .on_select(platform::run_on(this, move |_, _, cx| cx.emit(SidebarEvent::CloseGroup(group)))),
        Item::new(if pinned { "Unpin group" } else { "Pin group" })
            .on_select(platform::run_on(this, move |sidebar, _, cx| sidebar.toggle_group_pin(group, cx))),
        Item::new("Move group to new window").on_select(platform::run_on(this, move |_, _, cx| {
            cx.emit(SidebarEvent::MoveGroupToWindow(group))
        })),
    ]
}

/// The popover of the group `facts` describe, whose name is `name`.
pub(super) fn group_popover(
    facts: GroupFacts,
    name: String,
    appearance: Appearance,
    this: &WeakEntity<Sidebar>,
) -> Popover {
    let group = facts.group;
    let (swatches, chosen, on_pick) = dots(&facts, true, this);
    let commit = {
        let this = this.clone();
        move |text: &str, _: &mut Window, cx: &mut App| {
            let text = text.to_string();
            this.update(cx, |sidebar, cx| {
                // A blank name keeps the old one.
                sidebar.view.groups.rename(group, &text);
                sidebar.changed(cx);
            })
            .ok();
        }
    };
    let focus_back = this.clone();
    Popover::new(Behavior::Transient, appearance)
        .row(Row::Field(Field {
            value: name.into(),
            label: "Group name".into(),
            focused: true,
            on_commit: std::rc::Rc::new(commit),
            on_cancel: None,
        }))
        .row(Row::Colours(Colours {
            swatches: platform::menu::Swatches { swatches, selected: chosen, on_pick },
            custom: Some(Custom {
                color: facts.custom,
                start: facts.shown.unwrap_or(START),
                on_change: follow(group, this),
            }),
        }))
        .row(Row::Separator)
        .row(Row::Actions(folder_items(group, facts.folder.as_deref(), this)))
        .row(Row::Separator)
        .row(Row::Actions(actions(group, facts.pinned, this)))
        // The keyboard goes back to the terminal, unless an action put it somewhere else.
        .on_close(move |_, cx| {
            focus_back
                .update(cx, |sidebar, cx| {
                    if sidebar.group_renaming.is_none() {
                        cx.emit(SidebarEvent::FocusTerminal);
                    }
                })
                .ok();
        })
}

/// The menu of the group `facts` describe; every item acts on `this`.
pub(super) fn group_menu(facts: GroupFacts, this: &WeakEntity<Sidebar>) -> Menu {
    let group = facts.group;
    let (swatches, chosen, on_pick) = dots(&facts, false, this);
    let (start, on_change) = (facts.custom.or(facts.shown).unwrap_or(START), follow(group, this));
    let menu = Menu::new()
        .item(Item::new("Rename…").on_select(platform::run_on(this, move |sidebar, window, cx| {
            sidebar.start_group_rename(group, window, cx)
        })))
        .separator()
        .swatches(swatches, chosen, move |ix, window, cx| on_pick(ix, window, cx))
        .item(
            Item::new("Other colour…")
                .checked(facts.custom.is_some())
                .on_select(move |window, cx| platform::pick_colour(start, on_change.clone(), window, cx)),
        )
        .item(
            Item::new(NO_COLOUR)
                .checked(facts.paint.is_none())
                .on_select(platform::run_on(this, move |sidebar, _, cx| {
                    sidebar.set_group_color(group, None, cx)
                })),
        )
        .separator();
    let menu = folder_items(group, facts.folder.as_deref(), this).into_iter().fold(menu, Menu::item);
    actions(group, facts.pinned, this).into_iter().fold(menu.separator(), Menu::item)
}
