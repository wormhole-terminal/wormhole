//! A group's popover and context menu, from one description of the group (`GroupFacts`). The name
//! on its head opens the popover: the name in a field that has the keyboard at once, the nine
//! colours (the current one ringed) and a last dot that opens the system's Colours window for a
//! colour of one's own, then the actions. The right-click menu is Rename… (in place, `rename`),
//! the same dots, Other colour…, and the same actions.

use wormhole_core::sidebar::{GroupColor, GroupPaint};
use wormhole_core::tokens::Appearance;

use super::groups::group_colors;
use super::*;
use crate::native::popover::{Behavior, Colours, Custom, Field, Popover, Row};
use crate::native::{self, Item, Menu, Swatch};

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
    /// Every named colour on offer, with how the theme draws it.
    pub(super) colors: Vec<(GroupColor, Hsla)>,
}

impl Workspace {
    pub(super) fn group_facts(&self, group: u32, cx: &App) -> GroupFacts {
        let found = self.sidebar.view.groups.get(group);
        let paint = found.and_then(|g| g.paint());
        let tokens = &store_in(&self.profile, cx).tokens;
        GroupFacts {
            group,
            paint,
            custom: match paint {
                Some(GroupPaint::Custom([r, g, b])) => Some(from_bytes([r, g, b])),
                _ => None,
            },
            shown: paint.map(|paint| crate::theme_bridge::hsla(paint.color(tokens))),
            pinned: found.is_some_and(|g| g.pinned),
            colors: group_colors(&self.profile, cx),
        }
    }
}

fn from_bytes([r, g, b]: [u8; 3]) -> Hsla {
    Rgba { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a: 1.0 }.into()
}

/// The named colours as dots, which is ringed, and what a click on one does.
fn dots(
    facts: &GroupFacts,
    this: &WeakEntity<Workspace>,
) -> (Vec<Swatch>, Option<usize>, native::menu::Pick) {
    let swatches: Vec<Swatch> = facts
        .colors
        .iter()
        .map(|&(color, hsla)| Swatch { color: hsla, name: color.label().into() })
        .collect();
    let chosen = facts.colors.iter().position(|&(color, _)| facts.paint == Some(GroupPaint::Named(color)));
    let (group, colors, this) = (facts.group, facts.colors.clone(), this.clone());
    let on_pick: native::menu::Pick = std::rc::Rc::new(move |ix, _, cx| {
        if let Some(&(color, _)) = colors.get(ix) {
            this.update(cx, |ws, cx| ws.set_group_color(group, Some(color), cx)).ok();
        }
    });
    (swatches, chosen, on_pick)
}

/// What the Colours window does with each colour it sends: the group follows it.
fn follow(group: u32, this: &WeakEntity<Workspace>) -> native::ColourChange {
    let this = this.clone();
    std::rc::Rc::new(move |color, _, cx| {
        this.update(cx, |ws, cx| ws.set_group_custom_color(group, color, cx)).ok();
    })
}

/// The grey the Colours window starts at for a group with no colour.
const START: Hsla = Hsla { h: 0.0, s: 0.0, l: 0.5, a: 1.0 };

/// The actions both the popover and the menu end with, in their order.
fn actions(group: u32, pinned: bool, this: &WeakEntity<Workspace>) -> Vec<Item> {
    vec![
        Item::new("New terminal in group").on_select(native::run_on(this, move |ws, window, cx| {
            ws.new_session_in_group(group, window, cx)
        })),
        Item::new("Ungroup").on_select(native::run_on(this, move |ws, _, cx| ws.ungroup(group, cx))),
        Item::new("Close group")
            .on_select(native::run_on(this, move |ws, window, cx| ws.close_group(group, window, cx))),
        Item::new(if pinned { "Unpin group" } else { "Pin group" })
            .on_select(native::run_on(this, move |ws, _, cx| ws.toggle_group_pin(group, cx))),
        Item::new("Move group to new window").on_select(native::run_on(this, move |ws, window, cx| {
            ws.move_group_to_window(group, window, cx)
        })),
    ]
}

/// The popover of the group `facts` describe, whose name is `name`.
pub(super) fn group_popover(
    facts: GroupFacts,
    name: String,
    appearance: Appearance,
    this: &WeakEntity<Workspace>,
) -> Popover {
    let group = facts.group;
    let (swatches, chosen, on_pick) = dots(&facts, this);
    let commit = {
        let this = this.clone();
        move |text: &str, _: &mut Window, cx: &mut App| {
            let text = text.to_string();
            this.update(cx, |ws, cx| {
                // A blank name keeps the old one.
                ws.sidebar.view.groups.rename(group, &text);
                ws.mark_dirty(cx);
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
            swatches: native::menu::Swatches { swatches, selected: chosen, on_pick },
            custom: Some(Custom {
                color: facts.custom,
                start: facts.shown.unwrap_or(START),
                on_change: follow(group, this),
            }),
        }))
        .row(Row::Separator)
        .row(Row::Actions(actions(group, facts.pinned, this)))
        // The keyboard goes back to the terminal, unless an action put it somewhere else.
        .on_close(move |window, cx| {
            focus_back
                .update(cx, |ws, cx| {
                    if ws.sidebar.group_renaming.is_none() {
                        window.focus(&ws.active_terminal().focus_handle(cx), cx);
                    }
                })
                .ok();
        })
}

/// The menu of the group `facts` describe; every item acts on `this`.
pub(super) fn group_menu(facts: GroupFacts, this: &WeakEntity<Workspace>) -> Menu {
    let group = facts.group;
    let (swatches, chosen, on_pick) = dots(&facts, this);
    let (start, on_change) = (facts.custom.or(facts.shown).unwrap_or(START), follow(group, this));
    let menu =
        Menu::new()
            .item(Item::new("Rename…").on_select(native::run_on(this, move |ws, window, cx| {
                ws.start_group_rename(group, window, cx)
            })))
            .separator()
            .swatches(swatches, chosen, move |ix, window, cx| on_pick(ix, window, cx))
            .item(
                Item::new("Other colour…")
                    .checked(facts.custom.is_some())
                    .on_select(move |window, cx| native::pick_colour(start, on_change.clone(), window, cx)),
            )
            .separator();
    actions(group, facts.pinned, this).into_iter().fold(menu, Menu::item)
}
