//! The contract between the window and its features: what the window asks of a feature, as data and
//! function pointers, so that the window never names one. A feature is one [`FeatureDef`] in the
//! table the app installs ([`FeatureTable`]); core's `FEATURES` says what each is and whether it is on
//! (`wormhole_core::config::FeatureMask`, kept by the settings store), this says what it does.
//!
//! Two rules keep the borrows honest (a lease taken twice is a panic at run time):
//! 1. The window talks to a feature with `entity.update(cx, ..)`; a feature talks to the window by
//!    emitting an event the `Workspace` subscribed to when it made the entity. A feature never calls
//!    `workspace.update(..)` from inside its own update.
//! 2. What a feature needs to *read* of the window arrives as [`WindowFacts`](super::facts), borrowed,
//!    never as `&Workspace` (the transitional exception is at the end of this comment).
//!
//! A feature's state is one entity per window, made whether the feature is on or off (an off feature
//! keeps and records its data; only its views, keys and counts go). The window keeps it as an
//! [`AnyEntity`] and a hook downcasts it: once per hook per feature per frame, so hooks that apply to
//! rows or terminals take the whole set. A downcast clones a handle, which is a weak upgrade and an
//! atomic, and that is not a price to pay per row.
//!
//! Only Changes is in the table so far. Features move in one at a time (notes next), and a hook is
//! added when the first feature needs it, so a field below that nothing uses yet is the shape the
//! design settled on, not a promise that each is used.
//!
//! **Transitional.** Changes' state is still the `Workspace`'s fields and the `Session`'s, so its hooks
//! cannot do everything through an entity of their own yet. Where a hook has to read the window, it
//! reads the facts ([`WindowFacts::workspace`](super::facts::WindowFacts::workspace)); the four that
//! change it (`TabDef::view` and `on_show`, `WalkFns::press` and `fold`) are handed the `Workspace`
//! the window is already inside of (`ws`) and, for `view`, the window's `Context`. Both are the
//! window's own borrow passed down, so no lease is taken twice; they go with the last feature whose
//! state is still in the window (Changes, 5d), and a feature with an entity of its own ignores them.

#![cfg_attr(not(test), expect(dead_code, reason = "the contract is built before every feature uses it"))]

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::storage::store::SessionRecord;

use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::focus::{Activate, Walkable};
use crate::window::sidebar::RowExtras;

/// The features the app has, in the order of their tabs. The app installs it once at launch
/// ([`features::install`](super::features::install)) and each `Workspace` copies the slice into a field
/// when it is made, so the frame never reads the global.
#[derive(Clone, Copy)]
pub(in crate::window) struct FeatureTable(pub(in crate::window) &'static [FeatureDef]);

impl Global for FeatureTable {}

impl FeatureTable {
    pub(in crate::window) fn install(table: &'static [FeatureDef], cx: &mut App) {
        cx.set_global(Self(table));
    }

    /// The installed table. A window made before there is one (a test that doesn't need it) has
    /// no features, which is the table's own start.
    pub(in crate::window) fn installed(cx: &App) -> &'static [FeatureDef] {
        cx.try_global::<Self>().map_or(&[], |table| table.0)
    }
}

/// What a feature tells the window, as an event on its entity. The window owns this enum and
/// subscribes when it makes the entity ([`Workspace::listen`]), so a feature never holds the
/// `Workspace` and can't take its lease twice.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::window) enum FeatureEvent {
    /// Show this panel tab.
    ShowTab(TabId),
    /// Give the keyboard back to the terminal on screen.
    FocusTerminal,
    /// Something to save changed: the window's record is stale.
    Dirty,
    /// A text field of the feature has the keyboard (`true`) or let go of it. While one has it the
    /// panel's content is the focused area, as for a note being written.
    Editing(bool),
    /// Paste `text` into a terminal of this window.
    SendText { session: i64, text: SharedString },
}

/// A feature's entity and the window's hold on its events. Dropping it unsubscribes.
pub(in crate::window) struct FeatureState {
    pub entity: AnyEntity,
    pub _events: Subscription,
}

/// The hooks' shapes. Plain function pointers, never `dyn`: the frame calls them and a closure's box
/// would be an allocation.
pub(in crate::window) type BindFn = fn(&AnyEntity, &WindowFacts<'_>, Root, &App) -> Root;
pub(in crate::window) type RowExtrasFn = fn(&AnyEntity, &WindowFacts<'_>, &mut RowExtras<'_>, &App);
pub(in crate::window) type CloseStakeFn = fn(&AnyEntity, &[i64], &App) -> Option<Stake>;
pub(in crate::window) type MenuFn = fn(&AnyEntity, &WindowFacts<'_>, &mut MenuState, &App);
pub(in crate::window) type CountFn = fn(&AnyEntity, &WindowFacts<'_>, &App) -> Option<usize>;
pub(in crate::window) type TabActionsFn =
    fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App) -> Option<AnyElement>;
pub(in crate::window) type OnShowFn = fn(&AnyEntity, &mut Workspace, session: i64, &mut Context<Workspace>);
pub(in crate::window) type FocusInFn = fn(&AnyEntity, session: i64, &mut Window, &mut App) -> bool;
pub(in crate::window) type PressFn = fn(
    &AnyEntity,
    &mut Workspace,
    session: i64,
    row: usize,
    Activate,
    following: bool,
    &mut Window,
    &mut Context<Workspace>,
) -> bool;

/// The root element a feature adds its action handlers to.
pub(in crate::window) type Root = Div;

/// What a feature contributes to the window. Core's [`FeatureInfo`](wormhole_core::config::FeatureInfo)
/// of the same id says what it is called and whether it can be off.
pub(in crate::window) struct FeatureDef {
    pub id: FeatureId,
    /// The panel tabs it brings, in order. The table's order is the tab order, so there is no
    /// ordering field.
    pub tabs: &'static [TabDef],
    /// Binds the feature's own GPUI actions on the window's root (the tabs' `show` actions are the
    /// window's: it binds every [`TabDef::action`] itself). Actions are types, so a table cannot list
    /// them: the function adds its `on_action`s and decides when (Usage, only while an agent is on
    /// screen, which it reads from the facts). The closures capture the typed `Entity` the function
    /// downcasts, so an action updates the feature and never holds the `Workspace`'s lease. Called
    /// each frame while the feature is on, for a feature that has state.
    pub bind: Option<BindFn>,
    /// A row's extras (notes' open count, Changes' file count), for every row at once, through
    /// the narrow view the sidebar lends ([`RowExtras`]).
    pub row_extras: Option<RowExtrasFn>,
    /// What closing these terminals would lose (notes' open items, a worktree's changes), for the
    /// whole set being closed (a group, a window).
    pub close_stake: Option<CloseStakeFn>,
    /// Menu, palette and sheet state: actions hidden or disabled right now (Usage without an agent).
    /// Called whether the feature is on or off (the facts carry the mask) until the keymap's rows are
    /// tagged with their feature (5a).
    pub menu: Option<MenuFn>,
    /// The per-window state, if the feature has any.
    pub state: Option<StateDef>,
}

/// A panel tab of a feature.
pub(in crate::window) struct TabDef {
    pub id: TabId,
    pub icon: IconName,
    pub label: &'static str,
    /// The action that shows it: its key and its menu line are core's keymap's, and the window binds
    /// it ([`gpui_action`](crate::actions::gpui_action)) while the tab is offered.
    pub action: ActionId,
    /// What showing the tab does besides switching to it (Changes: refresh the repository), for the
    /// session shown. The window calls it from `show_tab`, after the switch.
    pub on_show: Option<OnShowFn>,
    /// Whether the tab is offered now (Usage: an agent on screen; Project: the terminal has a folder).
    pub shown: fn(&WindowFacts<'_>, &App) -> bool,
    /// A number beside the tab's name.
    pub count: Option<CountFn>,
    /// VoiceOver's words for the tab. A `String`: the tab bar asks once per tab for each frame.
    pub spoken: fn(&AnyEntity, &WindowFacts<'_>, &App) -> String,
    /// The tab's content, built inline from `Workspace::render` with the frame's facts: a panel as its
    /// own `Render` entity could not read the window, and pushing every fact into it would notify on
    /// each push. The per-frame list syncs run here, on the feature's entity. Its listeners capture the
    /// entity, not the `Workspace`.
    pub view: fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut Context<Workspace>) -> AnyElement,
    /// The keys over the tab's rows, for a tab that has a list. The window also gives such a tab the
    /// list area that holds the keyboard for it.
    pub walk: Option<WalkFns>,
    /// Buttons beside the tabs while it is on show (Notes' stash).
    pub actions: Option<TabActionsFn>,
}

/// F-70's four questions about a tab's rows, each over the feature's entity: a `&'static` table of
/// values could not hold the feature's folds and selection.
pub(in crate::window) struct WalkFns {
    /// The rows the keys can be on. A `Vec`: it is asked on key presses only.
    pub walkable: fn(&AnyEntity, &WindowFacts<'_>, &App) -> Vec<Walkable>,
    /// ↩ or Space on a row of `session` (`following`: a diff is open and follows the keys); whether
    /// the keys stay in the list.
    pub press: PressFn,
    /// → ← on a row.
    pub fold: fn(&AnyEntity, &mut Workspace, session: i64, row: usize, &mut Context<Workspace>),
    /// Scrolls a row into view.
    pub reveal: fn(&AnyEntity, &WindowFacts<'_>, row: usize, &App),
}

/// What closing some terminals would lose, in the words of the close question.
pub(in crate::window) struct Stake {
    /// How many things (notes' open items).
    pub count: usize,
    /// How the question says it ("3 open notes").
    pub sentence: String,
}

/// A feature's per-window state and how it follows the window's terminals. `SessionRecord` stays the
/// interchange format, so none of this changes what is saved.
pub(in crate::window) struct StateDef {
    /// Makes the entity with the window, whether the feature is on or off, and subscribes the window
    /// to its events ([`Workspace::listen`]). No handle to the `Workspace` is given to a feature:
    /// that would be a way to take its lease twice.
    pub make: fn(&mut Window, &mut Context<Workspace>) -> FeatureState,
    /// A terminal was selected: clear what the feature kept for the one before (Notes' focused note).
    pub selected: Option<fn(&AnyEntity, session: i64, &mut App)>,
    /// ⇧⌘E and the like: put the keys in the feature's field for this terminal; whether it took them.
    pub focus_in: Option<FocusInFn>,
    /// Writes the feature's part of a terminal's saved record.
    pub record: fn(&AnyEntity, session: i64, &mut SessionRecord, &App),
    /// Reads it back when the terminal is opened here, after `make`.
    pub opened: fn(&AnyEntity, &SessionRecord, &mut Window, &mut App),
    /// The terminal is closed for good: drop its state, and delete what the feature stores for it.
    pub closed: fn(&AnyEntity, session: i64, &mut App),
    /// The terminal lives on in another window: drop its state here and delete nothing. The other
    /// window gets it back through `record` and `opened`.
    pub left: fn(&AnyEntity, session: i64, &mut App),
}
