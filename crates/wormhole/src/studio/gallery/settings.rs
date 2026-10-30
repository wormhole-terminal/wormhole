//! The settings window's parts: the rows in their states under a group's head, the
//! nav items, and the controls that are ours.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::{BannerKind, NavMark, RecorderState, SettingParts, SettingState};

use super::Env;
use super::view::GalleryView;
use crate::settings::UiColors;
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge;

/// The settings content column and the nav, as wide as in the app.
const COLUMN_W: f32 = 560.0;
const NAV_W: f32 = 220.0;

pub(super) fn render(
    studio: &GalleryView,
    env: &Env,
    part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    // The page builds one piece at a time (a piece is a list item).
    match part {
        0 => div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(env.mark("setting.row", setting_rows(studio, env, cx)))
            .child(env.mark("settings.nav", nav(ui)))
            .into_any_element(),
        1 => controls(studio, env).into_any_element(),
        _ => div()
            .flex()
            .flex_col()
            .gap_6()
            .child(env.mark("banner", banners(ui)))
            .child(profile_marks(env))
            .into_any_element(),
    }
}

/// A profile's avatar at the sizes the app draws it, and the edge a window in another profile wears.
fn profile_marks(env: &Env) -> Div {
    let ui = env.ui;
    let colors = wormhole_core::view::sidebar::GroupColor::ALL
        .map(|color| (color, theme_bridge::hsla(color.color(env.tokens))));
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div().flex().items_center().gap_2().children(
                colors.iter().zip("DWPCSRABX".chars()).map(|((_, color), letter)| {
                    style::profile_avatar(ui, *color, letter, style::TITLE_GLYPH)
                }),
            ),
        )
        .child(
            div().flex().items_center().gap_2().children(
                colors.iter().zip("DWPCSRABX".chars()).take(3).map(|((_, color), letter)| {
                    style::profile_avatar(ui, *color, letter, PROFILE_AVATAR_LARGE)
                }),
            ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(style::choice_row("g-choice-copy", ui, "A copy of Default", true))
                .child(style::choice_row("g-choice-defaults", ui, "Wormhole's defaults", false)),
        )
        .child(div().flex().items_center().gap_3().child(style::profile_chip(
            "g-profile-chip",
            ui,
            colors[1].1,
            'W',
            "Work",
            "Profile Work, switch profile",
        )))
        .child(
            div()
                .w(px(POPOVER_SAMPLE_W))
                .p_1()
                .rounded(ui.radius_lg)
                .bg(ui.elevated)
                .outlined(ui, ui.border_strong)
                .child(style::popover_heading(ui, "Groups"))
                .child(style::popover_row(
                    "g-menu-project",
                    ui,
                    style::PopoverRow {
                        lead: Some(
                            style::color_dot(ui, colors[0].1, style::COLOR_DOT_MENU).into_any_element(),
                        ),
                        note: Some("3 terminals".into()),
                        ..style::PopoverRow::new("Project")
                    },
                ))
                .child(style::popover_row(
                    "g-menu-ungrouped",
                    ui,
                    style::PopoverRow {
                        note: Some("2 terminals".into()),
                        current: true,
                        ..style::PopoverRow::new("Ungrouped")
                    },
                ))
                .child(style::popover_rule(ui))
                .child(style::popover_row("g-menu-new-group", ui, style::PopoverRow::new("New group"))),
        )
        .child(
            div()
                .w(px(COLUMN_W))
                .rounded(ui.radius)
                .overflow_hidden()
                .outlined(ui, ui.border)
                .child(style::profile_edge(colors[4].1))
                .child(div().h(px(PROFILE_EDGE_SAMPLE_H)).bg(ui.background)),
        )
}

/// The larger avatar (the settings window's top) and the height of the window sample under the edge.
const PROFILE_AVATAR_LARGE: f32 = 24.0;
/// A sample of the app's rich popover rows.
const POPOVER_SAMPLE_W: f32 = 280.0;
const PROFILE_EDGE_SAMPLE_H: f32 = 24.0;

/// The setting row in each of its states.
fn setting_rows(studio: &GalleryView, env: &Env, cx: &App) -> Div {
    let ui = env.ui;
    let row = |id: &'static str,
               label: &'static str,
               description: &'static str,
               key: &'static str,
               state: SettingState,
               found: Option<std::ops::Range<usize>>,
               control: AnyElement| {
        style::setting(
            id,
            ui,
            SettingParts {
                label: label.into(),
                description: Some(description.into()),
                chip: None,
                key,
                state,
                found,
                found_description: None,
                control,
                tall: false,
                extra: None,
                on_reset: Some(Box::new(|_, _, _| {})),
            },
            cx,
        )
    };
    div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(COLUMN_W))
        .child(style::setting_group_head(ui, "Window", Some("A note at the right of the head")))
        .child(row(
            "g-setting-default",
            "Blur what's behind the window",
            "Only visible below 100 % opacity",
            "blur",
            SettingState::Default,
            None,
            studio.switch("g-setting-blur", "Blur what's behind the window", true, true, env),
        ))
        .child(row(
            "g-setting-changed",
            "Line height",
            "Terminal rows, as a multiple of the font size",
            "line_height",
            SettingState::Changed,
            None,
            style::stepper(ui, "g-setting-line", "1.45".into(), |_, _, _| {}, |_, _, _| {})
                .into_any_element(),
        ))
        .child(row(
            "g-setting-project",
            "Theme",
            "Set by api-server/.wormhole/theme.json while this terminal is active",
            "theme",
            SettingState::Project("api-server".into()),
            None,
            style::link_button("g-setting-edit", ui, "Edit in project", false).into_any_element(),
        ))
        .child(row(
            "g-setting-slider",
            "Background opacity",
            "30–100 %. Below 100 % the desktop shows through",
            "opacity",
            SettingState::Default,
            None,
            style::slider_control(ui, slider_track(ui), "80 %", &["30", "65", "100"], false)
                .into_any_element(),
        ))
        .child(row(
            "g-setting-link",
            "Font",
            "JetBrains Mono 14 pt · line height 1.30",
            "",
            SettingState::Default,
            None,
            style::link_button("g-setting-fonts", ui, "Edit in Fonts", true).into_any_element(),
        ))
        .child(row(
            "g-setting-found",
            "Wrap long lines",
            "Otherwise the diff scrolls sideways",
            "diff_wrap",
            SettingState::Default,
            Some(10..14),
            studio.switch("g-setting-wrap", "Wrap long lines", false, true, env),
        ))
        .child(row(
            "g-setting-disabled",
            "Terminal layout",
            "No layout.json in this project",
            "layout",
            SettingState::Disabled,
            None,
            studio.switch("g-setting-layout", "Terminal layout", false, false, env),
        ))
}

/// A slider's track and knob as drawn (the real one is GPUI Kit's, and needs a state).
fn slider_track(ui: UiColors) -> AnyElement {
    div()
        .relative()
        .w_full()
        .h(px(SLIDER_KNOB))
        .flex()
        .items_center()
        .child(div().w_full().h(px(SLIDER_TRACK)).rounded_full().bg(ui.border))
        .child(div().absolute().left_0().w(relative(0.7)).h(px(SLIDER_TRACK)).rounded_full().bg(ui.primary))
        .child(div().absolute().left(relative(0.66)).size(px(SLIDER_KNOB)).rounded_full().bg(ui.text))
        .into_any_element()
}

/// The drawn slider's track and knob.
const SLIDER_TRACK: f32 = 4.0;
const SLIDER_KNOB: f32 = 16.0;

/// The settings nav's group labels and items: normal, current, needs attention, search hits, none.
fn nav(ui: UiColors) -> Div {
    div()
        .flex()
        .flex_col()
        .w(px(NAV_W))
        .bg(ui.panel)
        .outlined(ui, ui.border)
        .p_2()
        .child(style::nav_group_label(ui, "Look"))
        .child(style::nav_row("g-nav-look", ui, IconName::Palette, "Appearance", false, NavMark::None))
        .child(style::nav_row("g-nav-fonts", ui, IconName::ALargeSmall, "Fonts", true, NavMark::None))
        .child(style::nav_group_label(ui, "Agents"))
        .child(style::nav_row(
            "g-nav-agents",
            ui,
            IconName::Sparkles,
            "Coding agents",
            false,
            NavMark::Attention,
        ))
        .child(style::nav_row(
            "g-nav-hits",
            ui,
            IconName::SquareTerminal,
            "Terminal",
            false,
            NavMark::Hits(1),
        ))
        .child(style::nav_row("g-nav-none", ui, IconName::ArrowDown, "Updates", false, NavMark::NoHits))
        .child(style::nav_row(
            "g-nav-preview",
            ui,
            IconName::FlaskConical,
            "Project folders",
            false,
            NavMark::Preview,
        ))
}

/// The settings controls that are ours rather than GPUI Kit's.
fn controls(studio: &GalleryView, env: &Env) -> Div {
    let ui = env.ui;
    let mode = |id: &'static str, icon, label: &'static str, active| {
        style::form_segment(id, ui, Some(icon), label, active)
    };
    let palette = [ui.primary, ui.success, ui.branch, ui.warning];
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_4()
                .child(
                    style::segmented_form(ui)
                        .child(style::form_segment("g-form-split", ui, None, "Split", true))
                        .child(style::form_segment("g-form-unified", ui, None, "Unified", false)),
                )
                .child(
                    style::segmented_form(ui)
                        .child(mode("g-form-dark", IconName::Moon, "Dark", false))
                        .child(mode("g-form-light", IconName::Sun, "Light", false))
                        .child(mode("g-form-system", IconName::Monitor, "Match system", true)),
                )
                .child(env.mark(
                    "stepper",
                    style::stepper(ui, "g-form-size", "14 pt".into(), |_, _, _| {}, |_, _, _| {}),
                )),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .children(
                    palette
                        .into_iter()
                        .enumerate()
                        .map(|(ix, color)| style::swatch(("g-swatch", ix), ui, color, ix == 0)),
                )
                .child(env.mark("tint.chip", style::tint_chip(ui, Some(IconName::Folder), "api-server")))
                .child(style::tint_chip(ui, None, "Bundled"))
                .child(style::changed_dot(ui))
                .child(style::keycap(ui, "⇧⌘P"))
                .child(style::keycap(ui, "esc"))
                .child(style::keycap(ui, "⌃⌘2")),
        )
        .child(
            style::preview_frame(ui).w(px(COLUMN_W)).child(
                style::preview_stage(ui, 56.0).child(
                    div()
                        .p_3()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("A preview's stage: its parts are placed by their distance from the top"),
                ),
            ),
        )
        .child(
            div().w(px(COLUMN_W)).child(
                style::preview_surface(ui)
                    .p_3()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("A preview's surface: the panel a list stands on, on its own"),
            ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(env.mark(
                    "recorder",
                    style::shortcut_recorder("g-rec-empty", ui, RecorderState::Empty, None),
                ))
                .child(style::shortcut_recorder(
                    "g-rec-set",
                    ui,
                    RecorderState::Set(vec!["⇧".into(), "⌘".into(), "Space".into()]),
                    None,
                ))
                .child(style::shortcut_recorder(
                    "g-rec-recording",
                    ui,
                    RecorderState::Recording(vec!["⌥".into()]),
                    None,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(style::shortcut_recorder(
                            "g-rec-conflict",
                            ui,
                            RecorderState::Conflict(vec!["⌘".into(), "Space".into()]),
                            None,
                        ))
                        .child(
                            div().text_caption().text_color(ui.warning_text).child("Spotlight uses ⌘Space"),
                        ),
                ),
        )
        .child(arranged(studio, env))
}

/// A list the person arranges (the title row's parts): a row that moves, the fixed space with its
/// note, and a hidden one, faint and still.
fn arranged(studio: &GalleryView, env: &Env) -> Div {
    let ui = env.ui;
    let reorder = |ix: usize, label: &'static str| style::Reorder {
        list: "g-arrange".into(),
        ix,
        label: label.into(),
        on_move: std::rc::Rc::new(|_, _, _, _| {}),
        dragged: false,
    };
    let switch = |id: &'static str, label: &'static str, on: bool| studio.switch(id, label, on, true, env);
    let row =
        |id: &'static str, icon, label: &'static str, note: Option<&'static str>, quiet: bool, on: bool| {
            let control = note.is_none().then(|| switch(id, label, on));
            let reorder = (!quiet).then(|| reorder(if note.is_some() { 1 } else { 0 }, label));
            style::arrange_row(
                id,
                ui,
                style::ArrangeParts {
                    icon,
                    label: label.into(),
                    note: note.map(Into::into),
                    quiet,
                    control,
                    reorder,
                },
            )
        };
    div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(COLUMN_W))
        .child(row("g-arrange-folder", IconName::Folder, "Folder", None, false, true))
        .child(row(
            "g-arrange-space",
            IconName::ArrowRightLeft,
            "Flexible space",
            Some("Left above, right below"),
            false,
            false,
        ))
        .child(row("g-arrange-ports", IconName::Globe, "Open ports", None, true, false))
}

/// Inline banners of each kind, and the flush one of a panel.
fn banners(ui: UiColors) -> Div {
    let small = |id: &'static str, label: &'static str, outline: bool| {
        Button::new(id).small().when(outline, |b| b.outline()).when(!outline, |b| b.ghost()).label(label)
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .max_w(px(COLUMN_W))
        .child(
            style::banner(
                ui,
                BannerKind::Info,
                false,
                "This terminal shows its project's theme. Your own is kept.",
            )
            .child(small("g-banner-info", "Project settings", false)),
        )
        .child(
            style::banner(
                ui,
                BannerKind::Warning,
                false,
                "config.json line 12: expected “,”. Using the last good settings.",
            )
            .child(small("g-banner-warning", "Open", true)),
        )
        .child(style::banner(
            ui,
            BannerKind::Danger,
            false,
            "Couldn't write ~/.gemini/settings.json: permission denied.",
        ))
        .child(style::banner(ui, BannerKind::Success, false, "Status hooks installed for Claude Code."))
        .child(style::banner(
            ui,
            BannerKind::Warning,
            true,
            "The settings file changed on disk. Reload to use it.",
        ))
}
