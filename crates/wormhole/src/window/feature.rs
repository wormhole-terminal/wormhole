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
//!    never as `&Workspace`.
//!
//! A feature's state is one entity per window, made whether the feature is on or off (an off feature
//! keeps and records its data; only its views, keys and counts go). The window keeps it as an
//! [`AnyEntity`] and a hook downcasts it: once per hook per feature per frame, so hooks that apply to
//! rows or terminals take the whole set. A downcast clones a handle, which is a weak upgrade and an
//! atomic, and that is not a price to pay per row.
//!
//! Features move in one at a time, and a hook is added when the first feature needs it, so a field below that nothing uses yet is the shape the
//! design settled on, not a promise that each is used.

#![cfg_attr(not(test), expect(dead_code, reason = "the contract is built before every feature uses it"))]

use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::SessionRecord;

use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature_host::FeatureState;
use crate::window::focus::{Activate, Walkable};
use crate::window::repo::Observed;
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
    /// panel's content is the focused area, as for a note being written. The window also ends it when
    /// the panel shows another tab, a feature is gone or another terminal is selected, because a field
    /// that loses the keyboard that way says nothing; so send `ShowTab` before `Editing(true)`.
    Editing(bool),
    /// Paste `text` into a terminal of this window and give it the keyboard; with `submit`, press
    /// Enter after it.
    SendText { session: i64, text: SharedString, submit: bool },
    /// This tab is gone (its feature went off): if the panel shows it, show Changes. Nothing else is
    /// done for the change, unlike a [`Self::ShowTab`].
    TabGone(TabId),
    /// Put `input` into a terminal of this window, as the `target` says which. The window checks nothing
    /// (not whether the input is a command a project may run): the feature that emits the event owns
    /// that decision. The terminals are found, or made, when the event is handled, so several in a row
    /// see each other's.
    Terminal { target: TerminalTarget, input: TerminalInput },
    /// Open these terminals, each in turn selected and given the keyboard, then select the last of them
    /// that asks for the focus. One whose name is already a terminal's is not opened again.
    OpenTerminals { terminals: Vec<NewTerminal> },
    /// The feature wants the window to read, or to read differently, what git says of a terminal's
    /// repository. The window owns that read (the sidebar's rows, the title row and the status bar
    /// show its answer), so a feature asks for it and finds the answer in the facts.
    Repo(RepoRequest),
}

/// What a feature asks of the window's reading of a terminal's repository ([`FeatureEvent::Repo`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::window) enum RepoRequest {
    /// Read the status now (a Retry, the refresh button, the tab being shown).
    Refresh { session: i64 },
    /// Read it against `branch` from now on (`None`: HEAD), and forget the answer for the old one.
    Compare { session: i64, branch: Option<String> },
}

/// Which terminal of the window a [`FeatureEvent::Terminal`] is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::window) enum TerminalTarget {
    /// The one on screen.
    Active,
    /// A new one, in `cwd`, selected.
    New { cwd: Option<PathBuf> },
    /// A new one in `cwd`, beside the selected terminal and in its group, selected and given the
    /// keyboard: what the New Terminal commands do.
    Beside { cwd: Option<PathBuf> },
    /// The one called `name`, made in `cwd` if there is none, and selected.
    Named { name: String, cwd: Option<PathBuf> },
}

/// What goes into the terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::window) enum TerminalInput {
    /// Nothing: the terminal is only found or made, and given the keyboard when it is a new one.
    None,
    /// Typed in, then Enter.
    Command(String),
    /// Pasted, then Enter a moment later with `submit`. The keyboard goes where the target puts it: a
    /// terminal the target makes or selects (`New`, `Beside`, `Named`) has it, the `Active` one keeps
    /// whatever had it.
    Paste { text: String, submit: bool },
}

/// A terminal [`FeatureEvent::OpenTerminals`] opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::window) struct NewTerminal {
    pub name: String,
    pub cwd: PathBuf,
    /// Typed in once it starts, then Enter.
    pub run: Option<String>,
    /// Selected at the end, of the ones opened.
    pub focus: bool,
}

/// The hooks' shapes. Plain function pointers, never `dyn`: the frame calls them and a closure's box
/// would be an allocation.
pub(in crate::window) type BindFn = fn(&AnyEntity, &WindowFacts<'_>, Root, &App) -> Root;
pub(in crate::window) type RowExtrasFn = fn(&AnyEntity, &WindowFacts<'_>, &mut RowExtras<'_>, &App);
pub(in crate::window) type CloseStakeFn = fn(&AnyEntity, &[i64], &App) -> Option<Stake>;
pub(in crate::window) type CloseExtraFn =
    fn(&AnyEntity, &WindowFacts<'_>, &[i64], CloseOf, &App) -> Option<CloseExtra>;
pub(in crate::window) type MenuFn = fn(&AnyEntity, &WindowFacts<'_>, &mut MenuState, &App);
pub(in crate::window) type CountFn = fn(&AnyEntity, &WindowFacts<'_>, &App) -> Option<usize>;
pub(in crate::window) type TabActionsFn =
    fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App) -> Option<AnyElement>;
pub(in crate::window) type OnShowFn = fn(&AnyEntity, &WindowFacts<'_>, session: i64, &mut App);
pub(in crate::window) type FocusInFn =
    fn(&AnyEntity, &WindowFacts<'_>, session: i64, &mut Window, &mut App) -> bool;
pub(in crate::window) type ProfileShownFn = fn(&mut App);
pub(in crate::window) type TickFn = fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App);
pub(in crate::window) type RefreshFn = fn(&AnyEntity, &WindowFacts<'_>, &mut App);
pub(in crate::window) type ApprovesProgramsFn = fn(&AnyEntity, session: i64, &ProfileId, &App) -> bool;
pub(in crate::window) type SpawnFn = fn(&ProfileId, Option<&Path>, &mut App) -> SpawnAdd;
pub(in crate::window) type PressFn = fn(
    &AnyEntity,
    &WindowFacts<'_>,
    session: i64,
    row: usize,
    Activate,
    following: bool,
    &mut Window,
    &mut App,
) -> bool;
pub(in crate::window) type FoldFn = fn(&AnyEntity, &WindowFacts<'_>, session: i64, row: usize, &mut App);
pub(in crate::window) type CoverCloseFn = fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App);
pub(in crate::window) type CoverTitleFn = fn(&AnyEntity, &WindowFacts<'_>, &mut App) -> Option<AnyElement>;
pub(in crate::window) type TabChangedFn = fn(&AnyEntity, TabId, &mut App);
pub(in crate::window) type PickBranchFn = fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App);
pub(in crate::window) type WatchFn = fn(&AnyEntity, &App) -> Option<PathBuf>;
pub(in crate::window) type ReadFn =
    fn(&AnyEntity, &WindowFacts<'_>, session: i64, Option<&Observed>, &mut App);

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
    /// What the feature adds to the question before terminals are closed.
    pub close: Option<CloseHooks>,
    /// Menu, palette and sheet state: actions disabled or hidden right now (Usage without an agent).
    /// Called whether the feature is on or off (the facts carry the mask). What an off feature hides
    /// is not this hook's: the keymap's rows are tagged with their feature, and the window hides the
    /// tagged ones of every feature that is off.
    pub menu: Option<MenuFn>,
    /// Once per window tick, for the state of every feature that has one, on or off: housekeeping that
    /// can't wait for an event (Notes throws away a note left empty that never had the keyboard).
    pub tick: Option<TickFn>,
    /// The shown profile's data is in `AppState` (at launch, after a profile switch): the feature reads
    /// its own app-wide data for it (Usage's reports). Per window state is the window's and doesn't
    /// need this. Called from outside any window, so it gets only the app.
    pub profile_shown: Option<ProfileShownFn>,
    /// The rest of what a feature that keeps data for each profile is told about the profiles.
    pub profiles: Option<ProfileHooks>,
    /// The window looks again at what it shows of the world outside the app (Project: which project each
    /// terminal is in): once in a while on the tick, when the window comes to the front, when its tab
    /// is shown, and when something every window should see changed.
    pub refresh: Option<RefreshFn>,
    /// Whether the folder of terminal `session` is one the person has approved to run programs for.
    /// Asked before something is started for the folder's sake (git's pull request lookup starts
    /// `gh`). The window fails closed: with no hook anywhere, nothing is approved.
    pub approves_programs: Option<ApprovesProgramsFn>,
    /// What a new terminal started in a folder gets from the feature (the project's environment and
    /// shell). Called for every feature, on or off: it decides itself. One user so far, noted as the
    /// exception to "a hook when a second feature needs it": the terminal is made by the window, and
    /// what it is made with is not the window's to know.
    pub spawn: Option<SpawnFn>,
    /// Something the feature lays over the terminal while it has it (Changes' open diff), and its part in
    /// the window's reading of the repository that diff is of.
    pub cover: Option<CoverDef>,
    /// The panel's tab was shown (any tab, also the one already on show): a feature drops what only
    /// belongs to its own tab being on show (a branch picker that is open).
    pub tab_changed: Option<TabChangedFn>,
    /// The title row's branch chip: what a click on it opens (Changes' "compare with").
    pub branch_chip: Option<BranchChip>,
    /// The per-window state, if the feature has any.
    pub state: Option<StateDef>,
}

impl FeatureDef {
    /// A feature that contributes nothing, for a table entry to build on in a `const`
    /// (`FeatureDef { id, tabs, ..FeatureDef::NONE }`), so an entry lists only what it brings and a
    /// hook added to the contract later touches only the features that use it. The id is a stand-in:
    /// every entry names its own.
    pub const NONE: Self = Self {
        id: FeatureId::Changes,
        tabs: &[],
        bind: None,
        row_extras: None,
        close: None,
        menu: None,
        tick: None,
        profile_shown: None,
        profiles: None,
        refresh: None,
        approves_programs: None,
        spawn: None,
        cover: None,
        tab_changed: None,
        branch_chip: None,
        state: None,
    };
}

/// What a feature lays over the terminal: the card of a diff. The window draws it in place of the
/// terminal and its panes, gives the title row the feature's entry in place of the breadcrumb, takes
/// the find shortcuts and the tab bar's focus stop away while it is up, and has Esc close it before
/// anything else. What it lays over is a file of the terminal's repository, so it also takes part in the
/// window's reading of that repository (`window::repo`).
pub(in crate::window) struct CoverDef {
    /// Whether the feature has something over the terminal now. Asked each frame and at keys: a read.
    pub open: fn(&AnyEntity, &App) -> bool,
    /// The card, built inline from `Workspace::render`.
    pub view: fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App) -> AnyElement,
    /// The title row's entry while it is up (where the file sits among its siblings).
    pub title: CoverTitleFn,
    /// Esc, and the feature's own close buttons: takes it down and gives the keyboard to the terminal.
    pub close: CoverCloseFn,
    /// A file to look at in the same background hop as the status, when the read is for the terminal
    /// on screen: what an open diff of the working tree watches.
    pub watch: WatchFn,
    /// The window read the status of terminal `session` (on screen); `observed` is what the hop saw of
    /// the file `watch` named. The answer is in the facts. Called whether or not something is laid over
    /// the terminal now: it is how an open diff finds out that its file changed.
    pub read: ReadFn,
}

/// The title row's branch chip. The window draws it (it knows the row's width) and hands a click to
/// the feature that contributed it, so the row names neither the feature nor its tab.
pub(in crate::window) struct BranchChip {
    /// The panel tab the click shows first, so what it opens is in view.
    pub tab: TabId,
    /// Opens the branch picker the chip stands for.
    pub pick: PickBranchFn,
}

/// A panel tab of a feature.
pub(in crate::window) struct TabDef {
    pub id: TabId,
    pub icon: IconName,
    pub label: &'static str,
    /// The action that shows it: its key and its menu line are core's keymap's, and the window binds
    /// it ([`gpui_action`](crate::actions::gpui_action)) while the tab is offered.
    pub action: ActionId,
    /// What showing the tab does besides switching to it, for the session shown. The window calls it
    /// from `show_tab`, after the switch. It takes no `Window`: `show_tab` is called from places that
    /// have none (a notification, a title chip's click handler off the render path).
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
    pub view: fn(&AnyEntity, &WindowFacts<'_>, &mut Window, &mut App) -> AnyElement,
    /// The keys over the tab's rows, for a tab that has a list. The window also gives such a tab the
    /// list area that holds the keyboard for it.
    pub walk: Option<WalkFns>,
    /// Buttons beside the tabs while it is on show (Notes' stash).
    pub actions: Option<TabActionsFn>,
    /// Redraw the window every this many ticks (400 ms each) while the tab is on show, for a tab whose
    /// words change with time and no event (Usage's "resets 34m").
    pub refresh_ticks: Option<u64>,
    /// The tab is offered only while something holds (Usage: an agent is on screen). The window
    /// remembers the tab it came from when this one is opened, and goes back there when it is gone.
    pub returns_to_previous: bool,
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
    pub fold: FoldFn,
    /// Scrolls a row into view.
    pub reveal: fn(&AnyEntity, &WindowFacts<'_>, row: usize, &App),
    /// The list the rows are in, for what scrolls it from outside (devctl's wheel); `None` while the
    /// terminal on screen has none.
    pub list: fn(&AnyEntity, &WindowFacts<'_>, &App) -> Option<ListState>,
}

/// What a feature adds to the start of a terminal: variables for its environment and the shell it
/// runs, which a later source (the profile's own environment, the terminal's own shell) may replace.
#[derive(Default, Debug, PartialEq, Eq)]
pub(in crate::window) struct SpawnAdd {
    pub env: std::collections::BTreeMap<String, String>,
    pub shell: Option<String>,
}

/// How a feature that keeps data for each profile follows the profiles: when one is opened beside the
/// shown one, when one is deleted, and when a terminal id is handed out in one (so data kept under an
/// id of a session that is gone can be dropped).
/// A feature registers only the hooks it needs.
pub(in crate::window) struct ProfileHooks {
    pub opened: Option<fn(&ProfileId, &mut App)>,
    pub deleted: Option<fn(&ProfileId, &mut App)>,
    pub id_allocated: Option<fn(&ProfileId, id: i64, &mut App)>,
}

impl ProfileHooks {
    /// No hook, for a table entry to build on in a `const` (`..ProfileHooks::DEFAULT`).
    pub const DEFAULT: Self = Self { opened: None, deleted: None, id_allocated: None };
}

/// A feature's part in the question before terminals are closed: both hooks see the whole set being
/// closed (a group, a window), and a feature registers the one it needs.
pub(in crate::window) struct CloseHooks {
    /// What closing these terminals would lose (notes' open items, a worktree's changes).
    pub stake: Option<CloseStakeFn>,
    /// What closing these terminals adds to the question the window asks: a sentence and, if the feature
    /// can do something more than closing, a button for it (a worktree Wormhole made may go with its
    /// terminal). Asked of every feature that is on; any extra makes the window ask even when nothing
    /// would be lost otherwise.
    pub extra: Option<CloseExtraFn>,
}

impl CloseHooks {
    /// No hook, for a table entry to build on in a `const` (`..CloseHooks::NONE`).
    pub const NONE: Self = Self { stake: None, extra: None };
}

/// What is being closed, for a feature's part in the question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::window) enum CloseOf {
    /// One terminal (its row's ×, ⌘W).
    Terminal,
    /// A group's terminals.
    Group,
}

/// What a feature adds to the question before closing terminals.
pub(in crate::window) struct CloseExtra {
    /// Said after the rest of the question's detail.
    pub note: String,
    /// A button between the closing one and Cancel, which closes and then does more.
    pub also: Option<CloseAlso>,
}

/// An extra button of the close question: its words, and what to do once the terminals are closed.
pub(in crate::window) struct CloseAlso {
    pub label: &'static str,
    pub then: std::rc::Rc<dyn Fn(&mut App)>,
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
    /// Makes the entity with the window, whether the feature is on or off, and returns what
    /// [`Workspace::listen`] makes of it: the entity and the window's subscription to its events. No
    /// handle to the `Workspace` is given to a feature: that would be a way to take its lease twice.
    pub make: fn(&mut Window, &mut Context<Workspace>) -> FeatureState,
    /// A terminal was selected: clear what the feature kept for the one before (Notes' focused note).
    pub selected: Option<fn(&AnyEntity, session: i64, &mut App)>,
    /// ⇧⌘E and the like: put the keys in the feature's field for this terminal; whether it took them.
    /// It gets the facts, for the panel's width a new field is made for.
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
