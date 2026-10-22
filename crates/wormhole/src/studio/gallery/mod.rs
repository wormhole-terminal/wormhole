//! The Studio's component gallery: every element Wormhole draws, in each state,
//! rendered with the live (previewed) tokens, in two columns that a filter narrows to one
//! part of the app. What the selected token is drawn on is outlined (dashed, the primary)
//! so an edit shows where it lands. Clicking a swatch or a labelled element selects its
//! token. Each specimen is one file; `SPECIMENS` lists them in page order.

mod ambient;
mod buttons;
mod chrome;
mod diff;
mod find;
mod inputs;
mod labels;
mod layout;
mod metrics;
mod navigation;
mod panel;
mod panes;
mod selects;
mod settings;
mod shape;
mod sidebar;
mod syntax;
mod terminal;
mod tiles;
mod typography;
mod view;
mod windows;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens::TOKEN_USES;
use wormhole_core::theme::tokens::{self, ThemeTokens};

use crate::settings::UiColors;
use crate::style::{self, TextRole};
use crate::workspace::studio::t;

pub(super) use view::GalleryView;

/// What a specimen draws with: the interface colors, the resolved tokens, and what the
/// selected token is used by.
struct Env<'a> {
    ui: UiColors,
    tokens: &'a ThemeTokens,
    /// The ids (core `TOKEN_USES`) of what the selected token is drawn on.
    uses: Vec<&'static str>,
}

impl Env<'_> {
    fn tk(&self, name: &str) -> Hsla {
        t(self.tokens, name)
    }

    /// `element`, outlined while the selected token is drawn on it. `id` is one of core's
    /// `TOKEN_USES`: the token editor's "Used by" names the same things.
    fn mark(&self, id: &'static str, element: impl IntoElement) -> AnyElement {
        debug_assert!(TOKEN_USES.iter().any(|u| u.id == id), "{id} is not in TOKEN_USES");
        div()
            .relative()
            .child(element)
            .when(self.uses.contains(&id), |d| d.child(style::uses_outline(self.ui, self.ui.radius)))
            .into_any_element()
    }
}

/// The part of the app a specimen belongs to: the gallery's filter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Area {
    #[default]
    All,
    Sidebar,
    Panel,
    Chrome,
    Settings,
    Ambient,
}

impl Area {
    pub(super) const ALL: [Area; 6] =
        [Area::All, Area::Sidebar, Area::Panel, Area::Chrome, Area::Settings, Area::Ambient];

    pub(super) fn label(self) -> &'static str {
        match self {
            Area::All => "All",
            Area::Sidebar => "Sidebar",
            Area::Panel => "Panel",
            Area::Chrome => "Chrome",
            Area::Settings => "Settings",
            Area::Ambient => "Ambient",
        }
    }

    /// The part of the app core's `TOKEN_USES` names, which the token list keeps to; `None` for
    /// All.
    pub(super) fn part(self) -> Option<tokens::Area> {
        Some(match self {
            Area::All => return None,
            Area::Sidebar => tokens::Area::Sidebar,
            Area::Panel => tokens::Area::Panel,
            Area::Chrome => tokens::Area::Chrome,
            Area::Settings => tokens::Area::Settings,
            Area::Ambient => tokens::Area::Ambient,
        })
    }
}

/// Which of the two columns a specimen stands in, or both: under them, the specimens that need
/// a settings window's or the quick panel's width.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Column {
    Left,
    Right,
    Both,
}

/// One section of the gallery page.
struct Specimen {
    title: &'static str,
    description: &'static str,
    /// Where it is shown besides All; `None` for the basics (type, terminal, code, sizes).
    area: Option<Area>,
    column: Column,
    /// The gap between this specimen's pieces: what its parts were separated by when they were one.
    part_gap: f32,
    /// About how tall each piece `render` makes is (points), until it has been built and measured:
    /// where pieces lie is worked out from these, so that the ones in view are built and the rest
    /// are spaces. A guess that is off is corrected as the page is scrolled, nothing worse.
    parts: &'static [f32],
    /// One piece: a tall specimen comes in several, so that only the ones in view are built.
    render: fn(&GalleryView, &Env, usize, &mut Context<GalleryView>) -> AnyElement,
}

const SPECIMENS: &[Specimen] = &[
    Specimen {
        title: "Sidebar",
        description: "bands, rows in every state, glyphs, status bar",
        area: Some(Area::Sidebar),
        column: Column::Left,
        part_gap: 16.0,
        parts: &[345.0, 527.0, 216.0, 571.0, 210.0, 217.0, 217.0, 163.0],
        render: sidebar::render,
    },
    Specimen {
        title: "Tiles and tabs",
        description: "status tiles, a card, the tab bars' tabs, chips, piles and segments",
        area: Some(Area::Sidebar),
        column: Column::Left,
        part_gap: 0.0,
        parts: &[847.0],
        render: tiles::render,
    },
    Specimen {
        title: "Right panel",
        description: "section bands, items, files, commits, icon tabs",
        area: Some(Area::Panel),
        column: Column::Right,
        part_gap: 16.0,
        parts: &[419.0, 328.0, 132.0, 395.0, 113.0],
        render: panel::render,
    },
    Specimen {
        title: "Diff",
        description: "header, hunk, context, removed, added, filler",
        area: Some(Area::Panel),
        column: Column::Right,
        part_gap: 0.0,
        parts: &[180.0],
        render: diff::render,
    },
    Specimen {
        title: "Chrome",
        description: "icons, icon buttons, the × and hit areas",
        area: Some(Area::Chrome),
        column: Column::Left,
        part_gap: 0.0,
        parts: &[483.0],
        render: chrome::render,
    },
    Specimen {
        title: "Find",
        description: "the find bar, matches and the current one",
        area: Some(Area::Chrome),
        column: Column::Both,
        part_gap: 0.0,
        parts: &[177.0],
        render: find::render,
    },
    Specimen {
        title: "Panes",
        description: "pane headers, focused and not, and the dividers",
        area: Some(Area::Chrome),
        column: Column::Both,
        part_gap: 0.0,
        parts: &[397.0],
        render: panes::render,
    },
    Specimen {
        title: "Tabs and chips",
        description: "tabs, chips, badges, rows, cards, tooltips",
        area: Some(Area::Chrome),
        column: Column::Left,
        part_gap: 0.0,
        parts: &[513.0],
        render: navigation::render,
    },
    Specimen {
        title: "Labels",
        description: "titles, section labels, empty notes, toolbars",
        area: Some(Area::Chrome),
        column: Column::Left,
        part_gap: 0.0,
        parts: &[188.0],
        render: labels::render,
    },
    Specimen {
        title: "Shape",
        description: "radii, the focus ring, everything chip-shaped",
        area: Some(Area::Chrome),
        column: Column::Left,
        part_gap: 0.0,
        parts: &[175.0],
        render: shape::render,
    },
    Specimen {
        title: "Settings",
        description: "rows, nav, controls, banners",
        area: Some(Area::Settings),
        column: Column::Both,
        part_gap: 24.0,
        parts: &[654.0, 368.0, 705.0],
        render: settings::render,
    },
    Specimen {
        title: "Buttons",
        description: "variants and sizes",
        area: Some(Area::Settings),
        column: Column::Right,
        part_gap: 0.0,
        parts: &[180.0],
        render: buttons::render,
    },
    Specimen {
        title: "Inputs",
        description: "fields, checkboxes and switches",
        area: Some(Area::Settings),
        column: Column::Right,
        part_gap: 0.0,
        parts: &[126.0],
        render: inputs::render,
    },
    Specimen {
        title: "Menus",
        description: "the select and the menu it opens",
        area: Some(Area::Settings),
        column: Column::Right,
        part_gap: 0.0,
        parts: &[187.0],
        render: selects::render,
    },
    Specimen {
        title: "Windows",
        description: "dialog, first-run sheet, About, Licenses",
        area: Some(Area::Settings),
        column: Column::Right,
        part_gap: 0.0,
        parts: &[710.0],
        render: windows::render,
    },
    Specimen {
        title: "Notifications",
        description: "quick panel and palette rows, readout",
        area: Some(Area::Ambient),
        column: Column::Both,
        part_gap: 0.0,
        parts: &[648.0],
        render: ambient::render,
    },
    Specimen {
        title: "Terminal",
        description: "16 ANSI, text, selection, cursor",
        area: None,
        column: Column::Left,
        part_gap: 0.0,
        parts: &[176.0],
        render: terminal::render,
    },
    Specimen {
        title: "Code",
        description: "highlighting from the syntax tokens",
        area: None,
        column: Column::Left,
        part_gap: 0.0,
        parts: &[124.0],
        render: syntax::render,
    },
    Specimen {
        title: "Type",
        description: "the text roles and colors",
        area: None,
        column: Column::Right,
        part_gap: 0.0,
        parts: &[293.0],
        render: typography::render,
    },
    Specimen {
        title: "Sizes and motion",
        description: "every size, limit, duration and easing",
        area: None,
        column: Column::Right,
        part_gap: 24.0,
        parts: &[657.0, 627.0],
        render: metrics::render,
    },
];

/// How many pieces the whole page is made of (the counting tests compare what a frame builds
/// with it).
#[cfg(test)]
pub(super) fn piece_count() -> usize {
    SPECIMENS.iter().map(|s| s.parts.len()).sum()
}

/// Narrower than this, the gallery is one column.
const TWO_COLUMNS_MIN_W: f32 = 680.0;

/// A specimen under its label and what it shows.
#[track_caller]
fn section(ui: UiColors, title: &'static str, description: &'static str, content: AnyElement) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_baseline()
                .gap_2()
                .child(style::section_label(ui, title))
                .child(div().min_w_0().truncate().text_caption().text_color(ui.faint).child(description)),
        )
        .child(framed(content))
        .into_any_element()
}

/// What a specimen is drawn in: whatever it can't fit in its column is cut off rather than drawn
/// over the other column; the inset leaves room for the uses outline round its edges.
#[track_caller]
fn framed(content: AnyElement) -> Div {
    div().w_full().min_w_0().overflow_hidden().p_1().child(content)
}

#[cfg(test)]
mod tests {
    use wormhole_core::theme::tokens::{self, TOKEN_USES};

    use super::Area;

    /// Every thing a token can be used by (the token editor's "Used by") is drawn and marked
    /// in the gallery, so selecting any token outlines all of its uses.
    #[test]
    fn every_use_is_marked_in_the_gallery() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/studio/gallery");
        let mut text = String::new();
        for entry in std::fs::read_dir(&dir).expect("the gallery's folder") {
            let path = entry.expect("an entry").path();
            if path.extension().is_some_and(|e| e == "rs") {
                text.push_str(&std::fs::read_to_string(&path).expect("a specimen"));
            }
        }
        // rustfmt may break the call after its parenthesis.
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ").replace("mark( ", "mark(");
        let missing: Vec<&str> =
            TOKEN_USES.iter().map(|u| u.id).filter(|id| !text.contains(&format!("mark(\"{id}\""))).collect();
        assert!(missing.is_empty(), "not marked in the gallery: {}", missing.join(", "));
    }

    /// Core's area of a use is the area of the specimen that draws it, so the token list under an
    /// area shows what the gallery shows there. Read from `SPECIMENS` and each specimen's file.
    #[test]
    fn each_use_is_drawn_in_its_areas_specimen() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/studio/gallery");
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).expect("a gallery file");
        let index = read("mod.rs");
        let table = index.split("const SPECIMENS").nth(1).expect("SPECIMENS");
        let mut checked = 0;
        for specimen in table.split("Specimen {").skip(1) {
            let field = |key: &str| {
                specimen.split(key).nth(1).and_then(|rest| rest.split(',').next()).map(str::trim).expect(key)
            };
            let area = match field("area:") {
                "None" => tokens::Area::Basics,
                some => {
                    let label = some.trim_start_matches("Some(Area::").trim_end_matches(')');
                    let area = Area::ALL.into_iter().find(|a| format!("{a:?}") == label).expect(label);
                    area.part().expect("a part")
                }
            };
            let file = field("render:").trim_end_matches("::render");
            let text = read(&format!("{file}.rs")).split_whitespace().collect::<Vec<_>>().join(" ");
            let text = text.replace("mark( ", "mark(");
            for id in text.split("mark(\"").skip(1).filter_map(|rest| rest.split('"').next()) {
                let found = TOKEN_USES.iter().find(|u| u.id == id).expect(id);
                assert_eq!(found.area, area, "{id} is drawn in {file}.rs");
                checked += 1;
            }
        }
        assert!(checked >= TOKEN_USES.len(), "only {checked} marks found");
    }
}
