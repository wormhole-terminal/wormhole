//! The settings window: the nav on the left (title, search, the sections in four
//! groups), a section's page on the right. Opens with ⌘,. Typing in the search shows the
//! settings it finds as live rows; Esc clears the search, then closes the window.
//!
//! By file: `open` where it opens and how a row from the palette is shown, `window` the window as a
//! view (its page and what its controls do), `nav` the left column, `sections` the pages and
//! `registry`, `recorder`, `font_picker` and `preview_column` what the pages share.
//!
//! Above the window and the shared state: it may use them; the app root, the palette and the status
//! item use it. The shared state keeps its page untyped.

#[cfg(test)]
mod close_tests;
pub(crate) mod devctl;
mod font_picker;
#[cfg(test)]
mod frame_tests;
#[cfg(test)]
mod motion_tests;
mod nav;
mod open;
mod preview_column;
mod recorder;
mod registry;
mod sections;
mod window;
#[cfg(test)]
mod words_tests;

use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName;
use gpui_kit::component::color_picker::ColorPickerState;
use gpui_kit::component::input::InputState;
use gpui_kit::component::slider::{SliderState, SliderValue};
use gpui_kit::*;
use wormhole_core::platform::paths;
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::view::sidebar::Density;

use crate::app_state::AppState;
use crate::platform::vibrancy::Blur;
use crate::settings::{UiColors, settings};
use crate::style::{self};
use font_picker::FontPickerState;

use open::current_window;
pub(crate) use open::{open_settings, reveal_row};
pub(crate) use sections::open_new_profile_sheet;
pub(crate) use wormhole_core::config::settings_index::SectionId as Section;

/// Where a page's title row starts: under the traffic lights, which the nav runs beneath like
/// System Settings.
const PAGE_TOP: f32 = style::TITLE_ROW_H;
/// The content column: the widest a page grows.
const COLUMN_W: f32 = 760.0;
/// A page's intro line wraps at a readable length, not across the whole column.
const INTRO_W: f32 = 560.0;
/// The card that says nothing matched.
const NO_RESULTS_W: f32 = 420.0;
/// The window: the size of a page (the nav and the content column) and the least the window
/// shrinks to. It opens wider than the page by the preview column (`preview_column`), to the
/// right, so the page keeps its width.
const WINDOW_SIZE: (f32, f32) = (920.0, 720.0);
const WINDOW_MIN: (f32, f32) = (640.0, 420.0);

/// What the Terminals & sidebar preview shows beside the current look while the pointer is
/// over another option.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SidebarLook {
    Density(Density),
}

/// The settings window's colours: an opaque window keeps the theme's own text colours, a
/// translucent one the main window's. It shadows `settings::ui` for everything drawn in this
/// window, whose files import it.
fn ui(cx: &App) -> UiColors {
    crate::settings::window_ui(cx, settings(cx).settings_translucent)
}

/// What the pages show of the window in front: its terminal's project (the "set by this project"
/// marks). Read when Settings opens and once a
/// second after, never in a frame: GPUI makes an entity read while a window draws that window's,
/// so the workspace would run its tick in Settings, and lose it when Settings closed.
#[derive(Clone, Debug, Default, PartialEq)]
struct FrontWindow {
    project: Option<PathBuf>,
}

impl FrontWindow {
    fn read(cx: &App) -> Self {
        AppState::front_window(cx)
            .and_then(|window| {
                window
                    .workspace
                    .read_with(cx, |ws, cx| Self {
                        project: crate::features::project::api::active_root(ws, cx),
                    })
                    .ok()
            })
            .unwrap_or_default()
    }
}

pub(crate) struct SettingsWindow {
    section: Section,
    /// The window in front, as the last refresh read it.
    front: FrontWindow,
    search: Entity<InputState>,
    /// The search shows its results page; picking a section from it shows that section with
    /// the matches marked, until the search changes again.
    show_results: bool,
    font_picker: FontPickerState,
    /// Motion → the four previews, there while the page is.
    motion_previews: Option<sections::MotionPreviews>,
    /// A shortcut field waiting for keys, or showing keys it refused.
    recording: Option<recorder::Recording>,
    /// Appearance → the "+" after the primary swatches: any color.
    primary_picker: Entity<ColorPickerState>,
    /// Appearance → the "+" after the branch color's swatches: any color.
    branch_picker: Entity<ColorPickerState>,
    /// The select whose menu is open (by its row id), and the search box the select menus share.
    open_select: Option<&'static str>,
    menu_search: Entity<InputState>,
    opacity_slider: Entity<SliderState>,
    line_height_slider: Entity<SliderState>,
    /// Fonts → which font the specimen shows.
    specimen: FontSlot,
    /// Appearance → every theme, not just the first rows.
    themes_all: bool,
    /// Terminals & sidebar → the option under the pointer, previewed beside the current look.
    sidebar_preview: Option<SidebarLook>,
    /// The window is wide enough for the preview column (read at each draw): the previews are in
    /// it, and their pages leave them out.
    preview_column: bool,
    /// Advanced → "Reset to defaults…" was pressed: the row asks before it resets.
    confirm_reset: bool,
    /// Coding agents → the agents that aren't on PATH are listed.
    others_open: bool,
    /// Profile → the fields of the profile Wormhole is in (made again when it changes).
    profile_form: Option<sections::ProfileForm>,
    /// Profile → "Delete profile…" was pressed: the row asks before it deletes.
    confirm_delete_profile: bool,
    /// The "New profile" sheet over the window.
    new_profile: Option<sections::NewProfileSheet>,
    /// Something that went wrong on a page (an import, writing an agent's hooks), shown at the
    /// top of that page until the page changes.
    notice: Option<(Section, SharedString)>,
    /// The page's scroll position: a page opens at its top.
    content_scroll: ScrollHandle,
    /// The row the palette asked for (`reveal_row`): marked, and its anchor in the page, until
    /// another section or a search takes over.
    revealed: Option<(&'static str, ScrollAnchor)>,
    /// The revealed row still has to be scrolled to (once the page is laid out).
    reveal_pending: bool,
    projects: sections::ProjectPicker,
    theme_picker: sections::ThemePicker,
    /// What the Agents page shows; read in the background (see `AgentsSnapshot`).
    agents: Option<sections::AgentsSnapshot>,
    agents_task: Option<Task<()>>,
    /// Import → the terminals found, which is open and what is ticked; scanned when the page is
    /// first shown.
    import: sections::ImportPage,
    /// Coding agents → Command line tool: where it stands.
    cli_tool: sections::CliToolPanel,
    /// The window's rem size as of the last draw, for boxes that are sized in rems and drawn
    /// from a cache (the theme cards).
    rem: Pixels,
    blur: Blur,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    /// Keeps the project picker's list current.
    _refresh: Task<()>,
}

/// The user's home folder, asked each time: the pages that show a path as `~/…` or
/// look under it are cold code, so nothing is cached.
fn home() -> Option<PathBuf> {
    paths::home()
}

/// `dir` as `~/…` when it is under the home folder, as the default location is shown.
fn with_tilde(dir: &Path) -> String {
    match home().as_deref().and_then(|home| dir.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => dir.display().to_string(),
    }
}

/// The icon the settings nav gives a section (the palette's settings show it too).
pub(crate) fn section_icon(section: Section) -> IconName {
    nav::section_icon(section)
}

/// A slider over `range` in `step`s, starting at `value`.
fn slider(min: f32, max: f32, step: f32, value: f32, cx: &mut App) -> Entity<SliderState> {
    cx.new(|_| SliderState::new().min(min).max(max).step(step).default_value(value))
}

fn slider_value(value: &SliderValue) -> f32 {
    match value {
        SliderValue::Single(value) => *value,
        SliderValue::Range(_, end) => *end,
    }
}

/// Show one of our folders in the Finder (the icon themes, the fonts),
/// creating it first so there's somewhere to drop files.
// GPUI's test platform panics on `open_with_system` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a view
// test that clicks this needs a stand-in first.
fn open_folder(dir: &Path, cx: &App) {
    match wormhole_core::storage::files::create_dir_all_private(dir) {
        Ok(()) => cx.open_with_system(dir),
        Err(err) => tracing::warn!("opening {}: {err}", dir.display()),
    }
}
