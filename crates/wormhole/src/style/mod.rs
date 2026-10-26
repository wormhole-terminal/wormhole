//! Wormhole's design language: the scales every view draws from and the few
//! shared components built on them. Views compose these instead of picking
//! their own numbers, so a new panel looks like the old ones by default. The
//! Theme Studio's gallery shows every component here in each of its states.
//!
//! **Spacing** is a 4-point grid, spelled with GPUI's rem-based helpers so it
//! also follows the interface font size: `_1` (4) between tight siblings,
//! `_2` (8) for the usual gap and inset, `_3` (12) around a panel's content,
//! `_4` (16) between groups and as the inset of every edge-to-edge row, `_6` (24)
//! around a page. Half steps (`_0p5`, `_1p5`) and `_5` are off the grid; a test
//! keeps them out.
//!
//! **Lists** run edge to edge: square rows and bands, inset [`SIDEBAR_INSET`] /
//! [`PANEL_INSET`] (12, the window's `EDGE_INSET`), one lead column at x 12–28 ([`ITEM_LEAD_W`]: a status
//! glyph, a chevron, a checkbox, a file icon) and the text at x 36. Heights are
//! [`BAND_H`] 32 (a folder band with a second line, [`FOLDER_BAND_H`], is 44 like a
//! two-line row), [`ROW_TWO_LINE_H`] 44 and [`ROW_H`] 28. "Current" is the
//! primary's tint, hover a 5 % fill; only choice controls (segmented, the active
//! tab chip) are filled with the solid primary.
//!
//! **Type** has four roles ([`TextRole`]): caption 12, body 14, heading 16,
//! title 20, plus `text_label` 13 / 600 for band titles. Row names are body in
//! 500, second lines caption.
//!
//! **Shape** comes from the theme's metrics via [`UiColors`]; borders are
//! `border_width` wide. A corner radius is picked by what an element *is*, so
//! everything of one kind matches and a theme can reshape a kind at once:
//!
//! | Radius | Role | Used by |
//! |---|---|---|
//! | `radius_chip` | chips: a short label in a box | [`pill`] (tabs, choice chips), badges, [`keycap`], [`segment`], branch and tint chips |
//! | `radius_sm` | tiny elements | [`close_x`], color swatches |
//! | `radius` | controls | buttons, [`icon_button`], [`icon_tab`], inputs, selects, banners, inset [`row`]s |
//! | `radius_lg` | cards and floating surfaces | [`card`], menus, popovers, theme cards |
//! | none | lists | [`flush_row`], [`session_row`], [`nav_row`], [`band`], flush banners |
//! | full | dots | agent status dots |
//!
//! `radius_chip` follows `radius_sm` unless the theme sets it. A new chip-like
//! component uses `radius_chip` too, and joins the chips in the Studio gallery.
//!
//! **Icons** are [`ICON_SIZE`] (14) wherever they stand on their own ([`icon`]); an
//! [`icon_button`] is [`HIT_MIN`] square with a 14 pt glyph, 16 in a title row.
//!
//! **Controls** come in two heights, [`CONTROL_H`] and the compact
//! [`CONTROL_H_SM`] used inside panels. Buttons are GPUI Kit's `Button`: `small`
//! in forms, `xsmall` in panels, `primary` for the one main action; a glyph-only
//! button is an [`icon_button`], never a text character. Anything smaller than
//! [`HIT_MIN`] that you click gets a [`hit_area`].
//!
//! **Motion** is in [`motion`]: durations, easings, the spinner and the pulse.
//!
//! **Settings** pages are built from [`setting_group`] and [`setting_row`] with the
//! [`toggle`], [`choice`] chips and [`stepper`] as controls (see the settings window's
//! registry, which lists every setting as data); [`setting`] is the row.
//!
//! The helpers that build an element are `#[track_caller]`, so the inspector
//! (⇧⌥⌘I) points at the view that used them rather than at this file.

mod bands;
mod buttons;
mod checkbox;
mod controls;
mod extras;
mod find_bar;
mod folder_colors;
mod groups;
mod home;
mod icon_motion;
mod labels;
mod lights;
pub(crate) mod menu_layer;
mod messages;
mod metrics;
pub mod motion;
mod palette;
mod panel;
pub(crate) mod panel_list;
mod panes;
mod popover;
pub(crate) mod popover_layer;
mod preview;
mod profile;
mod reorder;
mod rolling_number;
mod rows;
mod scale;
mod section;
mod setting;
mod size_readout;
mod splits;
mod status;
mod status_shift;
mod submodule;
mod tab_selection;
mod tabs;
mod tile;
mod tint;
mod title_drag;
mod window_surfaces;

pub use bands::*;
pub use buttons::*;
pub use checkbox::*;
pub use controls::*;
pub use extras::*;
pub use find_bar::*;
pub use folder_colors::*;
pub use groups::*;
pub(crate) use home::home_dir;
pub use icon_motion::{Half, IconMotion, MenuMark, StripFill, StripSide};
pub use labels::*;
pub(crate) use lights::{ASSUMED_LIGHTS_SPAN, LIGHTS_X, lights_end, traffic_light_position};
pub use messages::*;
pub use metrics::Metrics;
pub use palette::*;
pub use panel::*;
pub use panes::*;
pub use popover::*;
pub use preview::*;
pub use profile::*;
pub use reorder::*;
pub use rolling_number::*;
pub use rows::*;
pub use scale::*;
pub use section::*;
pub use setting::*;
pub use size_readout::*;
pub use splits::*;
pub use status::*;
pub use status_shift::*;
pub use submodule::*;
pub use tab_selection::*;
pub use tabs::*;
pub use tile::*;
pub(crate) use title_drag::TitleDrag;
pub(crate) use window_surfaces::{drag_strip, wants_blur, window_surfaces};
