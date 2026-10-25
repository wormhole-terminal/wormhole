//! The scales: sizes and insets, the type roles and the theme-driven borders.

use std::sync::{Arc, LazyLock};

use gpui_kit::component::Size;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;

pub use wormhole_core::view::layout::status_bar::STATUS_BAR_H;
pub use wormhole_core::view::limits::{
    COUNT_BADGE_MAX, DIFF_SPLIT_MIN, DONE_FOLD_ABOVE, FOLD_ABOVE, NOTE_OPEN_MAX_LINES, STATUS_BAR_SIZE_MIN,
    STATUS_BAR_WORDS_MIN, count_label,
};

// ---------------------------------------------------------------- heights

/// Height of a form control (input, select, small button) and of an icon button.
pub const CONTROL_H: f32 = 28.0;
/// Height of a control inside a panel (tab, chip, compact select).
pub const CONTROL_H_SM: f32 = 22.0;
/// A one-line list row: a task, a note, a file, a nav item, a compact terminal.
pub const ROW_H: f32 = 28.0;
/// A two-line row: a terminal with a second line, a commit, a project action. Four
/// above, [`NAME_LINE`], [`META_LINE`], four below.
pub const ROW_TWO_LINE_H: f32 = 44.0;
/// The name line of a two-line row, in rems (20 at the default interface size).
pub const NAME_LINE: f32 = 1.25;
/// The second line of a two-line row, in rems (16).
pub const META_LINE: f32 = 1.0;
/// The settings nav's group label with the step above it: 12 of space, a caption line, 4 below.
pub const NAV_GROUP_H: f32 = 32.0;
/// A folder band in the sidebar, a section band in the right panel.
pub const BAND_H: f32 = 32.0;
/// A folder band with its second line (the folder's path, ±N and branch): the height of a
/// two-line row, a name line and a meta line between the same four above and below. "No folder"
/// and the compact density keep [`BAND_H`].
pub const FOLDER_BAND_H: f32 = ROW_TWO_LINE_H;
/// The status tile (`style::status_tile`) beside a two-line row's two lines, and the small one in a
/// tab and the title row.
pub const TILE: f32 = 32.0;
pub const TILE_MINI: f32 = 20.0;
/// The other sizes of the tile: the title row's, and a tab's, a dense row's and the status bar's.
/// A dense folded head's is [`TILE_DENSE_FOLD`]; a compact row's and a folded head's is
/// [`TILE_MINI`].
pub const TILE_TITLE: f32 = 18.0;
pub const TILE_SMALL: f32 = 16.0;
pub const TILE_DENSE_FOLD: f32 = 14.0;
/// The rounded selection and hover inside a card.
pub const CARD_ROW_RADIUS: f32 = 8.0;
/// The tab bar (terminals as tabs): every tab's width; too many to fit, and the row scrolls.
pub const TAB_W: f32 = 172.0;
/// The title rows across the top: the sidebar's, the terminal's breadcrumb, the panel's tabs. The
/// traffic lights (14 pt) are centred in it, which puts them 12 pt from the window's top as they are
/// from its left: the terminal's surface stands as far from the window's sides and bottom.
pub const TITLE_ROW_H: f32 = 38.0;
/// Height of a badge (one caption line). Its least width is the same, so a lone
/// digit sits in a square.
pub const BADGE_H: f32 = 16.0;
/// A key cap: a caption line with room around it, at least this wide so ⌘1 and esc match.
pub const KEYCAP_H: f32 = 20.0;
pub const KEYCAP_MIN_W: f32 = 28.0;
/// Below this a setting row grows no shorter: a label, its description and room.
pub const SETTING_ROW_MIN_H: f32 = 48.0;
/// An inline banner's least height: one line of text beside a small button.
pub const BANNER_MIN_H: f32 = 40.0;

// ---------------------------------------------------------------- insets and columns

/// The one standard inset from a column's (or the window's) edge to its content: the traffic lights,
/// the sidebar's header, the title row's first item, the terminal's text, the status bar's ends, the
/// tab bar and the folder row all start this far in, so the window has shared edges.
pub const EDGE_INSET: f32 = 12.0;
/// The terminal's grid inside its surface (as in 1.1; a command block's mark lies in it), and the
/// surface's inset from the window's edge, the sidebar, the panel and the status bar: the two make
/// the [`EDGE_INSET`], so the text, not the surface, lines up with the traffic lights.
pub const TERMINAL_PAD: f32 = 4.0;
pub const SURFACE_INSET: f32 = EDGE_INSET - TERMINAL_PAD;

/// The left and right inset of every sidebar row, band and toolbar.
pub const SIDEBAR_INSET: f32 = EDGE_INSET;
/// The same inset in the right panel.
pub const PANEL_INSET: f32 = EDGE_INSET;
/// The lead column of a row (x 12–28): a status glyph, a chevron, a checkbox, a file
/// icon. Names start a gap (`_2`) after it, at x 36.
pub const ITEM_LEAD_W: f32 = 16.0;
/// How far each level of a tree (folders in Changes, files in a commit) steps in: one
/// lead column and its gap.
pub const INDENT: f32 = 24.0;
/// The room between the right panel's tab bar and what each tab shows.
pub const PANEL_LIST_TOP: f32 = 4.0;
/// How far in from the panel's edge a row inside a card has its content: the card's margin, its
/// hairline and padding (the sidebar's regular cards, `Metrics::default`), and the row's own inset.
pub const PANEL_ROW_X: f32 = 10.0 + 1.0 + 4.0 + 8.0;
/// The right padding of a header row that ends in an icon button (a tab's header in the
/// panel): the button's 16 pt glyph ends on the 12 pt inset, so the row stops 6 in.
pub const HEADER_PAD_END: f32 = PANEL_INSET - (HIT_MIN - TITLE_GLYPH) / 2.0;

// ---------------------------------------------------------------- glyphs and hit areas

/// Every icon the chrome draws on its own (14 pt). Icons inside a line of caption text
/// stay `xsmall`.
pub const ICON_SIZE: Size = Size::Small;
/// An icon button's glyph, and the larger one in a title row.
pub const GLYPH: f32 = 14.0;
pub const TITLE_GLYPH: f32 = 16.0;
/// The icon above an empty list's message ("No terminals match").
pub const EMPTY_GLYPH: f32 = 24.0;
/// The least size of anything clickable: a smaller drawing keeps its size and
/// gets a hit area this big around it ([`super::hit_area`]).
pub const HIT_MIN: f32 = 28.0;
/// An agent's status dot, with a halo this wide round it while it needs input.
pub const STATUS_DOT_SIZE: f32 = 8.0;
pub const STATUS_HALO: f32 = 3.0;
/// The dot that marks a changed setting, an active filter, a panel wanted but hidden.
pub const MARK_DOT: f32 = 6.0;
/// The gap between a control and its focus ring (the ring's width is the theme's
/// `metrics.focus_ring_width`). Rows draw theirs inside instead.
pub const FOCUS_RING_OFFSET: f32 = 1.0;
/// How opaque a row or band stays while it is dragged.
pub const DRAG_LIFT_OPACITY: f32 = 0.4;
/// The least the empty space under the sidebar's last row is, where a dragged terminal can be dropped.
pub const SIDEBAR_TAIL_MIN_H: f32 = 8.0;
/// Thickness of the line that shows where a dragged item will land.
pub const DROP_LINE_W: f32 = 2.0;
/// A resize handle straddles its hairline this wide.
pub const RESIZE_HANDLE_W: f32 = 8.0;
/// The gap between the terminals of a split when each is its own rounded surface (`split_gaps`); the
/// divider's handle fills it.
pub const PANE_GAP: f32 = 8.0;

// ---------------------------------------------------------------- widths

/// A branch chip never grows past this, and shrinks to no less (a glyph and three letters).
pub const BRANCH_CHIP_MAX: f32 = 112.0;
pub const BRANCH_CHIP_MIN: f32 = 56.0;
/// The share of a row a file name may take before it truncates in the middle.
pub const FILE_NAME_MAX: f32 = 0.55;
/// The share of a band a folder name may take before it truncates.
pub const FOLDER_NAME_MAX: f32 = 0.6;
/// The sidebar's filter popover.
pub const FILTER_POPOVER_W: f32 = 320.0;
/// A slider in a setting row, and in a narrow one.
pub const SLIDER_W: f32 = 200.0;
pub const SLIDER_W_COMPACT: f32 = 160.0;
/// A color swatch, and a small one (in a theme card's name strip).
pub const SWATCH: f32 = 24.0;
pub const SWATCH_SMALL: f32 = 12.0;
/// A folder colour's dot in a row of choices, and in front of its name in a menu.
pub const COLOR_DOT: f32 = 16.0;
pub const COLOR_DOT_MENU: f32 = 12.0;
/// A theme card in Settings → Appearance.
pub const THEME_CARD_W: f32 = 176.0;
pub const THEME_CARD_H: f32 = 88.0;
/// The shortcut recorder.
pub const RECORDER_W: f32 = 200.0;
/// The quick panel and the command palette: width, search row, result row, and how far
/// down the screen they open (a share of its height).
pub const QUICK_PANEL_W: f32 = 600.0;
pub const QP_SEARCH_H: f32 = 48.0;
pub const QP_ROW_H: f32 = 36.0;
pub const QP_TOP: f32 = 0.2;

/// The four type roles, plus the label role of band titles. Sizes are GPUI's rem
/// steps, so they scale with the interface font size.
pub trait TextRole: Styled + Sized {
    /// Metadata, badges, section labels, second lines (12).
    fn text_caption(self) -> Self {
        self.text_xs()
    }

    /// A folder band's name, a section band's title (13 / 600).
    fn text_label(self) -> Self {
        self.text_size(rems(0.8125)).font_weight(FontWeight::SEMIBOLD)
    }

    /// Content: list rows, form labels, buttons, paragraphs (14).
    fn text_body(self) -> Self {
        self.text_sm()
    }

    /// A subsection heading, a panel's title (16).
    fn text_heading(self) -> Self {
        self.text_base()
    }

    /// The top of a page (20).
    fn text_title(self) -> Self {
        self.text_xl()
    }

    /// Digits of one width, so counts and times in a column don't wobble.
    fn tabular_nums(self) -> Self {
        static TNUM: LazyLock<FontFeatures> =
            LazyLock::new(|| FontFeatures(Arc::new(vec![("tnum".into(), 1)])));
        self.font_features(TNUM.clone())
    }
}

impl<T: Styled> TextRole for T {}

/// Borders that follow the theme's `border_width` instead of a fixed pixel.
pub trait Edges: Styled + Sized {
    /// A border on every side.
    fn outlined(mut self, ui: UiColors, color: Hsla) -> Self {
        let w = ui.border_width.into();
        let b = &mut self.style().border_widths;
        (b.top, b.right, b.bottom, b.left) = (Some(w), Some(w), Some(w), Some(w));
        self.border_color(color)
    }

    fn rule_top(mut self, ui: UiColors) -> Self {
        self.style().border_widths.top = Some(ui.border_width.into());
        self.border_color(ui.border)
    }

    fn rule_bottom(mut self, ui: UiColors) -> Self {
        self.style().border_widths.bottom = Some(ui.border_width.into());
        self.border_color(ui.border)
    }

    /// A line along the bottom `lines` border widths thick in `color`: a project's colour under
    /// the title row, where it stands in for a tint that Increase Contrast takes away.
    fn rule_bottom_in(mut self, ui: UiColors, color: Hsla, lines: f32) -> Self {
        self.style().border_widths.bottom = Some((ui.border_width * lines).into());
        self.border_color(color)
    }

    fn rule_left(mut self, ui: UiColors) -> Self {
        self.style().border_widths.left = Some(ui.border_width.into());
        self.border_color(ui.border)
    }

    fn rule_right(mut self, ui: UiColors) -> Self {
        self.style().border_widths.right = Some(ui.border_width.into());
        self.border_color(ui.border)
    }
}

impl<T: Styled> Edges for T {}
