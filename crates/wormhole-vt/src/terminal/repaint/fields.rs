//! Every field of the engine's state and how a repaint covers it.
//!
//! `tests/structure.rs` reads the structs' source and compares their field names with this list,
//! so a field added to the engine without a line here fails the build: the author has to say
//! whether a repaint writes it, derives it from something it writes, leaves it to the host, or
//! cannot say it (and then which [`Gaps`](super::Gaps) bit says so).

/// How a field is covered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Written: the text names the step (the `Writer` method or the sequence).
    Written(&'static str),
    /// Rebuilt as a consequence of something that is written.
    Derived(&'static str),
    /// The host's own configuration or input, not state the program made.
    Host(&'static str),
    /// A queue the host drains; nothing a replica needs.
    Queue(&'static str),
    /// The viewer's: scroll position, selection, pins. A replica has its own.
    Viewer(&'static str),
    /// Bookkeeping that nothing observes.
    Scratch(&'static str),
    /// Cannot be written in VT; the text names the [`Gaps`](super::Gaps) bit, which the
    /// bisimulation test expects to fail for.
    Gap(&'static str),
}

/// A struct of the engine's state and the coverage of its fields.
#[derive(Clone, Copy, Debug)]
pub struct StructFields {
    /// The struct's name.
    pub name: &'static str,
    /// Its source file, relative to the crate's `src/`.
    pub file: &'static str,
    /// Each field, in the order the source declares them.
    pub fields: &'static [(&'static str, Coverage)],
}
use Coverage::{Derived, Gap, Host, Queue, Scratch, Viewer, Written};

const TERMINAL: &[(&str, Coverage)] = &[
    ("grid", Written("screen_rows: the screen showing, or the main one written first")),
    ("other", Written("screen_rows: the other screen, written after `?47h`")),
    ("cols", Host("the receiver is made at the sender's size")),
    ("rows", Host("the receiver is made at the sender's size")),
    ("cursor", Written("final_state: DECRSPS, the delayed wrap included")),
    ("pen", Written("final_state: SGR, OSC 8, DECSCA, SPA, OSC 133 (see Pen)")),
    ("modes", Written("modes: every flag but the ones below; see MODES")),
    ("recorded", Written("modes: the recorded DEC private modes")),
    ("saved_modes", Written("saved_modes: each is set and XTSAVEd, in order")),
    ("margins", Written("modes: DECSTBM and DECSLRM")),
    ("tabs", Written("tab_stops: DECRSPS 2, or HTS")),
    ("charsets", Written("cursor_information: DECRSPS")),
    ("locator", Written("locator: DECELR, DECSLE, DECEFR (the pointer is the host's)")),
    ("preferred", Written("misc: DECAUPSS")),
    ("saved", Written("saved_cursors: DECSC on each screen")),
    ("keys", Written("keyboard: CSI = u and CSI > u per screen")),
    ("cursor_style", Written("misc: DECSCUSR and mode 12")),
    ("titles", Written("titles: OSC 2, OSC 1, CSI > Ps t and the stack")),
    ("policy", Host("what the host allows; set on the receiver by its host")),
    ("limits", Host("the caps the host set; the receiver is made with its own")),
    ("window", Host("where the host's window is")),
    ("folder", Written("titles: OSC 7")),
    ("palette", Written("palette: OSC 4, 5, 10 to 19")),
    ("cell_px", Host("the host's cell size")),
    ("last_char", Written("touch_and_counters, or the open cluster printed again")),
    ("cluster", Written("final_state: the cluster is printed again (Gaps::CLUSTER when it cannot be)")),
    ("block", Written("Seeds::block, and OSC 133;A marks that count up from it")),
    ("stashed_content", Written("final_state: the main screen's pen mark is set, then `?47h` puts it aside")),
    ("link_seq", Written("Seeds::link_seq")),
    ("link_text", Scratch("a cache of the text of the last links, to share it; no sequence reads it")),
    ("identity", Host("what XTVERSION says; the receiver is made with its own")),
    ("parser", Scratch("in the ground state between sequences (repaint_ready)")),
    ("osc", Scratch("empty between sequences (repaint_ready)")),
    ("dcs", Scratch("empty between sequences (repaint_ready)")),
    ("passthrough", Host("the strings the host asked for; the receiver is made with its own")),
    ("graphics", Gap("Gaps::PLACEMENTS and IMAGES; the handle counter is Seeds::image_handle")),
    ("events", Queue("drained by the host")),
    ("replies", Queue("written to the program by the host")),
    ("reply_dropped", Scratch("the reply being written, one sequence's")),
    ("scrollback_lines", Host("the history setting")),
    (
        "history_just_cleared",
        Written("final_state: CSI 3 J last, when the history is empty (Gaps::HISTORY_CLEARED)"),
    ),
    ("level", Written("level: DECSCL first")),
    ("eight_bit", Written("final_state: S8C1T")),
    ("kept", Written("misc: the numbers programs set for hardware we do not have")),
    ("guard_used", Written("misc: SPA then EPA")),
    ("attr_extent", Written("misc: DECSACE, through `kept`")),
    ("select", Viewer("the selection and the word separators")),
    ("seen", Viewer("what the last take_damage saw of the cursor and the colours")),
    ("scheme", Host("the host's light or dark scheme; the receiver is told its own")),
    ("told", Queue("what the host was last told changed (ModesChanged, ColorsChanged)")),
];

const CURSOR: &[(&str, Coverage)] =
    &[("x", Written("DECRSPS")), ("y", Written("DECRSPS")), ("pending", Written("DECRSPS flags"))];

const MARGINS: &[(&str, Coverage)] = &[
    ("top", Written("DECSTBM")),
    ("bottom", Written("DECSTBM")),
    ("left", Written("DECSLRM")),
    ("right", Written("DECSLRM")),
];

const SAVED: &[(&str, Coverage)] = &[
    ("cursor", Written("DECRSPS then DECSC")),
    ("style", Written("SGR then DECSC")),
    ("protected", Written("DECSCA then DECSC")),
    ("origin", Written("DECRSPS flags then DECSC")),
    ("charsets", Written("DECRSPS then DECSC")),
    ("link", Written("OSC 8 then DECSC")),
];

const CLUSTER: &[(&str, Coverage)] = &[
    ("open", Written("the cluster is printed again")),
    ("head", Derived("where it is printed again")),
    ("width", Derived("printing it again")),
    ("last", Derived("printing it again")),
    ("state", Derived("printing it again")),
];

const GRID: &[(&str, Coverage)] = &[
    ("cols", Host("the receiver's size")),
    ("rows", Host("the receiver's size")),
    ("pages", Written("the rows, by RowView")),
    ("end", Scratch("the serial of the next row; only placements and pins read it")),
    ("limit", Host("the history cap")),
    ("offset", Viewer("how far the view is scrolled")),
    ("next_uid", Scratch("page ids")),
    ("spare", Scratch("a page kept for reuse")),
    ("pins", Viewer("selection anchors and search hits")),
    ("placements", Gap("Gaps::PLACEMENTS")),
    ("all_dirty", Viewer("what the snapshot must copy again")),
    ("page_rows", Scratch("a test knob")),
    ("archive", Host("the lines waiting for the host, if it asked for the history that is dropped")),
    ("segments", Scratch("scratch for a scroll")),
    ("copied", Scratch("scratch for DECCRA")),
    ("copied_marks", Scratch("scratch for DECCRA")),
];

const PEN: &[(&str, Coverage)] = &[
    ("style", Written("SGR")),
    ("link", Written("OSC 8")),
    ("content", Written("OSC 133 B, C, D and A;k=s")),
    ("block", Written("OSC 133;A counts up from the seed")),
    ("protected", Written("DECSCA")),
    ("guarded", Written("SPA and EPA")),
];

const ROW: &[(&str, Coverage)] = &[
    ("block", Written("OSC 133;A before the row's first marked cell")),
    ("slot", Scratch("where the row's cells are in the page")),
    ("flags", Written("the wrap by printing into the next row; DIRTY is the viewer's")),
];

const GRAPHICS: &[(&str, Coverage)] = &[
    ("apc", Scratch("empty between sequences")),
    ("transfer", Scratch("a picture in chunks: repaint_ready says no")),
    ("tables", Gap("Gaps::IMAGES")),
    ("last_handle", Written("Seeds::image_handle")),
    ("gate", Gap("replies held for a decode; the replica never answers")),
    ("multipart", Scratch("an iTerm2 image in parts: repaint_ready says no")),
];

const LOCATOR: &[(&str, Coverage)] = &[
    ("reporting", Written("DECELR")),
    ("units", Written("DECELR")),
    ("on_down", Written("DECSLE")),
    ("on_up", Written("DECSLE")),
    ("filter", Written("DECEFR")),
    ("pointer", Host("where the host's pointer is")),
    ("buttons", Host("which buttons the host's pointer has down")),
];

const CHARSETS: &[(&str, Coverage)] = &[
    ("slots", Written("DECRSPS designators")),
    ("gl", Written("DECRSPS")),
    ("gr", Written("DECRSPS")),
    ("single", Written("DECRSPS flags")),
    ("plain", Derived("recomputed from the others")),
];

const KEYBOARD: &[(&str, Coverage)] = &[("stack", Written("CSI = Ps ; 1 u, then CSI > Ps u"))];

const TABS: &[(&str, Coverage)] = &[("stops", Written("DECRSPS 2, or HTS"))];

const KEPT: &[(&str, Coverage)] = &[
    ("local_functions", Written("CSI Ps + q")),
    ("function_key_control", Written("CSI Ps * }")),
    ("status_display", Written("CSI Ps $ }")),
    ("modifier_keys", Written("CSI Ps + r")),
    ("status_line", Written("CSI Ps $ ~")),
    ("attribute_extent", Written("DECSACE")),
];

const CURSOR_STYLE: &[(&str, Coverage)] =
    &[("shape", Written("misc: DECSCUSR")), ("blink", Written("misc: DECSCUSR and mode 12"))];

const TITLES: &[(&str, Coverage)] = &[
    ("window", Written("titles: OSC 2")),
    ("icon", Written("titles: OSC 1")),
    ("modes", Written("titles: CSI > Ps t")),
    ("stack", Written("titles: set and XTWINOPS 22 for each slot")),
];

const FOLDER: &[(&str, Coverage)] = &[
    ("now", Written("titles: OSC 7")),
    ("reports", Queue("OSC 7 reports since the host looked")),
    ("reported", Queue("how many of the reports are in use")),
];

const PALETTE: &[(&str, Coverage)] = &[
    ("set", Written("palette: OSC 4, 10, 11, 12")),
    ("special", Written("palette: OSC 5")),
    ("extra", Written("palette: OSC 13 to 19")),
    ("defaults", Host("the host's palette, set on the receiver by SetColors")),
    ("touched", Scratch("a color command ran since the feed began; the end of the feed looks")),
];

const TOLD: &[(&str, Coverage)] = &[
    ("modes", Queue("the watched modes the host was told about")),
    ("cursor", Queue("the cursor's look the host was told about")),
    ("colors", Queue("the colors the host was told about")),
];

const SAVED_TITLES: &[(&str, Coverage)] =
    &[("window", Written("OSC 2 then XTWINOPS 22")), ("icon", Written("OSC 1 then XTWINOPS 22"))];

const STYLE: &[(&str, Coverage)] = &[
    ("fg", Written("SGR (Gaps::COLOR for a default or the cursor color as a cell's)")),
    ("bg", Written("SGR")),
    ("underline_color", Written("SGR 58")),
    ("attrs", Written("SGR")),
];

const PLACEMENT: &[(&str, Coverage)] =
    &[("spec", Gap("Gaps::PLACEMENTS")), ("top", Gap("Gaps::PLACEMENTS")), ("col", Gap("Gaps::PLACEMENTS"))];

/// The structs whose fields make up the engine's state.
pub const FIELDS: &[StructFields] = &[
    StructFields { name: "Terminal", file: "terminal/mod.rs", fields: TERMINAL },
    StructFields { name: "CursorPos", file: "terminal/mod.rs", fields: CURSOR },
    StructFields { name: "Margins", file: "terminal/mod.rs", fields: MARGINS },
    StructFields { name: "Saved", file: "terminal/mod.rs", fields: SAVED },
    StructFields { name: "Cluster", file: "terminal/mod.rs", fields: CLUSTER },
    StructFields { name: "CursorStyle", file: "terminal/mod.rs", fields: CURSOR_STYLE },
    StructFields { name: "Titles", file: "terminal/reports/title.rs", fields: TITLES },
    StructFields { name: "Folder", file: "terminal/osc/cwd.rs", fields: FOLDER },
    StructFields { name: "Palette", file: "terminal/osc/colors.rs", fields: PALETTE },
    StructFields { name: "Told", file: "terminal/watch.rs", fields: TOLD },
    StructFields { name: "Grid", file: "grid/mod.rs", fields: GRID },
    StructFields { name: "Pen", file: "grid/row.rs", fields: PEN },
    StructFields { name: "Row", file: "grid/row.rs", fields: ROW },
    StructFields { name: "Style", file: "grid/style.rs", fields: STYLE },
    StructFields { name: "Placement", file: "grid/images/mod.rs", fields: PLACEMENT },
    StructFields { name: "Graphics", file: "terminal/graphics/mod.rs", fields: GRAPHICS },
    StructFields { name: "Locator", file: "terminal/reports/locator.rs", fields: LOCATOR },
    StructFields { name: "Charsets", file: "terminal/text/charset/mod.rs", fields: CHARSETS },
    StructFields { name: "KeyboardFlags", file: "terminal/modes/keyboard.rs", fields: KEYBOARD },
    StructFields { name: "TabStops", file: "terminal/screen/tabs.rs", fields: TABS },
    StructFields { name: "Kept", file: "terminal/modes/kept.rs", fields: KEPT },
    StructFields { name: "SavedTitles", file: "terminal/reports/title.rs", fields: SAVED_TITLES },
];

/// Every mode flag and how a repaint covers it. A test checks it against `Modes`.
pub const MODES: &[(&str, Coverage)] = &[
    ("SHOW_CURSOR", Written("modes: DECSET 25")),
    ("AUTOWRAP", Written("modes: DECSET 7")),
    ("ALT_SCREEN", Derived("the screen that shows: `?47h` at the end")),
    ("ALTERNATE_SCROLL", Written("modes: DECSET 1007")),
    ("BRACKETED_PASTE", Written("modes: DECSET 2004")),
    ("FOCUS_REPORT", Written("modes: DECSET 1004")),
    ("APP_CURSOR", Written("modes: DECSET 1")),
    ("MOUSE_CLICK", Written("modes: DECSET 1000")),
    ("MOUSE_DRAG", Written("modes: DECSET 1002")),
    ("MOUSE_MOTION", Written("modes: DECSET 1003")),
    ("MOUSE_SGR", Written("modes: DECSET 1006")),
    ("MOUSE_UTF8", Written("modes: DECSET 1005")),
    ("KEYS_DISAMBIGUATE", Derived("the top of the screen showing's keyboard stack")),
    ("KEYS_EVENT_TYPES", Derived("the top of the screen showing's keyboard stack")),
    ("KEYS_ALTERNATE_KEYS", Derived("the top of the screen showing's keyboard stack")),
    ("KEYS_ALL_AS_ESCAPES", Derived("the top of the screen showing's keyboard stack")),
    ("KEYS_ASSOCIATED_TEXT", Derived("the top of the screen showing's keyboard stack")),
    ("INSERT", Written("modes: SM 4")),
    ("NEW_LINE", Written("modes: SM 20")),
    ("KEYBOARD_LOCK", Written("modes: SM 2")),
    ("SEND_RECEIVE", Written("modes: SM 12")),
    ("COLUMNS_132", Gap("Gaps::COLUMNS_132: DECCOLM only sets by resizing")),
    ("SMOOTH_SCROLL", Written("modes: DECSET 4")),
    ("REVERSE_VIDEO", Written("modes: DECSET 5")),
    ("ORIGIN", Written("cursor_information: DECRSPS flags")),
    ("AUTO_REPEAT", Written("modes: DECSET 8")),
    ("MOUSE_X10", Written("modes: DECSET 9")),
    ("LOCATOR", Derived("the Locator's own fields: `modes()` adds it while the locator is on")),
    ("ALLOW_COLUMNS", Written("modes: DECSET 40, always")),
    ("REVERSE_WRAP", Written("modes: DECSET 45")),
    ("APP_KEYPAD", Written("modes: DECSET 66")),
    ("BACKARROW_BS", Written("modes: DECSET 67")),
    ("LEFT_RIGHT_MARGINS", Written("modes: DECSET 69")),
    ("SIXEL_DISPLAY", Written("modes: DECSET 80")),
    ("NO_CLEAR_ON_COLUMNS", Written("modes: DECSET 95")),
    ("MOUSE_URXVT", Written("modes: DECSET 1015")),
    ("MOUSE_PIXELS", Written("modes: DECSET 1016")),
    ("SYNC_OUTPUT", Written("modes: DECSET 2026")),
    ("GRAPHEME_CLUSTERS", Written("modes: DECSET 2027 (the cells are written in whichever mode makes them)")),
    ("SIXEL_PRIVATE_PALETTE", Written("modes: DECSET 1070")),
    ("SIXEL_CURSOR_RIGHT", Written("modes: DECSET 8452")),
    ("THEME_REPORT", Written("modes: DECSET 2031")),
    ("SCROLL_ON_OUTPUT", Written("modes: DECSET 1010")),
    ("SCROLL_ON_KEY", Written("modes: DECSET 1011")),
    ("META_SENDS_ESC", Written("modes: DECSET 1036")),
    ("ALT_SENDS_ESC", Written("modes: DECSET 1039")),
    ("BELL_URGENCY", Written("modes: DECSET 1042")),
    ("REVERSE_WRAP_EXTENDED", Written("modes: DECSET 1045")),
    ("MORE_FIX", Written("modes: DECSET 41")),
];

/// The flags of a grid cell and how a repaint covers each. A test checks it against the cell's.
pub const CELL_FLAGS: &[(&str, Coverage)] = &[
    ("WIDE", Written("the wide character is printed")),
    ("SPACER_TAIL", Derived("printing the wide character")),
    ("SPACER_HEAD", Derived("the wide character on the next row wraps and writes it")),
    ("GRAPHEME", Written("the cluster's marks are printed after the base")),
    ("HYPERLINK", Written("OSC 8")),
    ("PROTECTED", Written("DECSCA")),
    ("IMAGE", Scratch("unused: placements are the grid's")),
    ("TAB", Written("HT on the blank cell")),
    ("GUARDED", Written("SPA and EPA")),
];
