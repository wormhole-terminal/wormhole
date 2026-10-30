//! The picker's panel: a field, a ghost completion, a breadcrumb and a list of rows, drawn by the system as a
//! panel of its own glass, whatever the rows are. It shows what a [`View`] says and reports what the person
//! does ([`Event`]); what the rows mean, what a key does and what a pick hands over are core's
//! (`view::picker`) and the app's (`crate::picker`).
//!
//! It is a native panel and not GPUI's drawing because the picker is a system-looking thing (Liquid Glass, the
//! system's search field, rows like a menu's, VoiceOver's list), and because the field it holds is the
//! system's own, so input methods, dictation and the spelling of the keys are the system's, not ours. The
//! panel is a child window of the app window's, which it follows and goes with, and it takes the keys (the
//! app window is not the key window while it is up) without activating the app, so it never comes over other
//! apps. See `macos::picker`.
//!
//! AppKit calls the panel's controls from its own event dispatch, while GPUI may have its `App` borrowed, so
//! nothing here runs app code: what happened goes in a queue ([`Events`]) that a foreground task of GPUI reads.
//! The one thing that cannot wait for the task is whether the picker takes an arrow key or leaves it to the
//! field, which AppKit asks before the key returns: the panel asks core's `decide` itself, with the
//! [`Facts`](wormhole_core::view::picker::Facts) it was last given.

use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::picker::{Card, Crumb, Do, Facts, Field, Key, Row};

pub use super::os::picker::Panel;

/// Where what the person did goes: the app reads it (`crate::picker`).
pub type Events = smol::channel::Sender<Event>;

/// Something the person did in the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The text in the field changed (typed, pasted, cleared), in a field that showed the picker's text of this
    /// epoch (`View::epoch`): an edit of text the picker has since replaced is told apart by it.
    Typed(Field, u64),
    /// What a key the picker takes was decided to do: the panel asked `decide` with the facts it had, before
    /// the key returned, and this is that answer, which the app does as it is.
    Do(Do),
    /// A key with the field as it was, for a caller that has no panel to have decided (a test, devctl): the
    /// picker decides.
    Key(Key, Field),
    /// A scripted click on a line of the list.
    #[cfg(any(test, feature = "devctl"))]
    Row(usize),
    /// A click on a row as it appeared when pressed.
    RowKey { at: usize, key: String },
    /// A secondary click on a row that offers a context menu.
    ContextKey { at: usize, key: String },
    /// The pointer entered a row; its key rejects events from a recycled or changed row.
    Hover { at: usize, key: String },
    /// A choice in the panel's optional segmented control.
    Segment(usize),
    /// A click on a step of the breadcrumb.
    Crumb(usize),
    /// The wheel or the trackpad: this many lines.
    Scroll(isize),
    /// The panel stopped being the key window (a click elsewhere, another app): the picker is done.
    Resigned,
}

/// A line of the list.
#[derive(Clone, Copy, Debug)]
pub enum Line<'a> {
    Header(&'a str),
    Row(&'a Row),
    Note(&'a str),
}

/// The optional filter above the list, owned by the picker rather than AppKit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segments {
    pub labels: Vec<String>,
    pub selected: usize,
    pub label: String,
    /// A short status next to the control, such as the number of results.
    pub status: String,
}

/// Everything the panel shows.
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    /// The lines in view, and nothing else: a list of hundreds costs the panel its few lines.
    pub lines: &'a [Line<'a>],
    pub segments: Option<&'a Segments>,
    /// Where `lines` start in the whole list: what a click on one reports (`Event::Row`).
    pub top: usize,
    /// The selected row's place in the whole list.
    pub selected: Option<usize>,
    pub crumbs: &'a [Crumb],
    /// The field's text, which replaces what is in it when `epoch` is not the one it showed last.
    pub text: &'a str,
    pub epoch: u64,
    pub ghost: Option<&'a str>,
    pub facts: Facts,
    pub placeholder: &'a str,
    /// What VoiceOver calls the panel and its list.
    pub name: &'a str,
    pub hint: &'a str,
    /// The theme's appearance, which the panel is drawn in whatever the system's is.
    pub appearance: Appearance,
}

/// The panel's metrics, AppKit's native ones for a list of this kind (the app's style scale is for what GPUI
/// draws; this is system chrome, like a menu row).
pub const WIDTH: f32 = 480.0;
pub const ROW: f32 = 28.0;
pub const MAX_ROWS: usize = 9;
/// Number of compact status indicators visible beside a row.
pub const MAX_INDICATORS: usize = 4;
/// What the panel has besides rows: its padding, the field, the hint line below the list, and the breadcrumb.
const CHROME: f32 = 76.0;
const CRUMBS: f32 = 26.0;
/// Space below the search field for an optional native segmented control.
const SEGMENTS: f32 = 30.0;

/// The panel as the placement rules (`view::picker::place`) need it.
pub fn card(has_crumbs: bool, has_segments: bool) -> Card {
    Card {
        width: WIDTH,
        chrome: CHROME + if has_crumbs { CRUMBS } else { 0.0 } + if has_segments { SEGMENTS } else { 0.0 },
        row: ROW,
        max_rows: MAX_ROWS,
    }
}

/// What the panel is made of.
pub use super::notice::Material;

/// How the panel stands, for devctl's `state`.
#[cfg(feature = "devctl")]
#[derive(Clone, Debug)]
pub struct Info {
    pub material: Material,
    /// The panel in the window's own space (points from its top left): x, y, width, height.
    pub frame: [f32; 4],
    /// The panel is the key window: the field has the keyboard.
    pub key: bool,
    /// The panel is on screen. A background-mode instance never shows it.
    pub visible: bool,
    pub window_number: i64,
    pub parent_number: i64,
    /// What the ghost label says, and the rows' labels in view, as the panel shows them.
    pub ghost: String,
    pub rows: Vec<String>,
    /// The AppKit font names applied to the visible row labels.
    pub row_fonts: Vec<Option<String>>,
    pub crumbs: Vec<String>,
    /// Labels, selected index and status shown by the optional native filter.
    pub segments: Option<(Vec<String>, usize, String)>,
}
