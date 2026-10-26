//! Appearance: the live preview, the mode and the theme cards, the primary and the branch
//! color as swatches, the window's opacity, blur and status bar, the title row's parts
//! (`title_row`), the file icons. Under Reduce
//! Transparency the opacity slider is off and says why; with the blur off below 95 % the
//! blur's row warns.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerState};
use gpui_kit::component::slider::Slider;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, SectionId as Section};
use wormhole_core::config::{BranchColor, ThemeMode, TitleBorder};
use wormhole_core::theme::tokens::{BranchTint, Color, ThemeTokens, Translucency};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{open_button, stage, title_row};
use crate::select::Choice;
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, UiColors, settings};
use crate::settings_window::registry::{ProjectSource, Segment, Setting, toggle};
use crate::settings_window::{SettingsWindow, open_folder, ui};
use crate::style::{self, Edges};
use crate::theme_bridge;

const MODES: &[Segment] = &[
    Segment::icon(IconName::Moon, SETTINGS_APPEARANCE_DARK),
    Segment::icon(IconName::Sun, SETTINGS_APPEARANCE_LIGHT),
    Segment::icon(IconName::Monitor, SETTINGS_APPEARANCE_MATCH_SYSTEM),
];

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("appearance-preview", stage::draw_stage),
    Setting::segmented(
        "theme-mode",
        MODES,
        |s| match s.theme_mode {
            ThemeMode::Dark => 0,
            ThemeMode::Light => 1,
            ThemeMode::System => 2,
        },
        |s, ix| {
            s.theme_mode = match ix {
                0 => ThemeMode::Dark,
                1 => ThemeMode::Light,
                _ => ThemeMode::System,
            }
        },
    ),
    Setting::block("theme", draw_theme),
    Setting::toggle("primary-color", |s| s.uses_theme_primary(), |s, on| s.theme_primary = Some(on))
        .describe(describe_primary)
        .below(draw_swatches),
    Setting::segmented(
        "branch-color",
        &[
            Segment::text(SETTINGS_APPEARANCE_BRANCH_THEME),
            Segment::text(SETTINGS_APPEARANCE_BRANCH_PRIMARY),
            Segment::text(SETTINGS_APPEARANCE_BRANCH_CUSTOM),
        ],
        |s| BranchColor::ALL.iter().position(|c| *c == s.branch_color).unwrap_or(0),
        |s, ix| s.branch_color = BranchColor::ALL.get(ix).copied().unwrap_or_default(),
    )
    .describe(describe_branch)
    .below(draw_branch_swatches),
    Setting::custom("opacity", draw_opacity).below(reduced_transparency_note),
    toggle!("blur", blur).below(blur_off_warning),
    Setting::custom("translucent", draw_translucent),
    Setting::segmented(
        "title-border",
        &[
            Segment::text(SETTINGS_APPEARANCE_ALWAYS),
            Segment::text(SETTINGS_APPEARANCE_WHEN_SCROLLED),
            Segment::text(SETTINGS_APPEARANCE_NEVER),
        ],
        |s| TitleBorder::ALL.iter().position(|b| *b == s.title_border).unwrap_or(1),
        |s, ix| s.title_border = TitleBorder::ALL.get(ix).copied().unwrap_or_default(),
    ),
    toggle!("column-dividers", column_dividers),
    toggle!("status-bar", status_bar),
    Setting::custom("title-row", title_row::draw_control).below(title_row::draw_parts),
    Setting::custom("icon-theme", draw_icon_select),
    Setting::custom("icon-themes-folder", |_, _| {
        open_button(
            "icon-themes-folder",
            IconName::ExternalLink,
            text(SETTINGS_APPEARANCE_OPEN_FOLDER),
            |cx| open_folder(&crate::icons::user_dir(), cx),
        )
    }),
];

/// "The theme's · #7aa2f7. Off: …": the primary in use, the theme's or the one chosen.
fn describe_primary(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let store = cx.global::<SettingsStore>();
    let base = store.base_tokens().get("primary").map(|c| c.to_hex()).unwrap_or_default();
    let (_, _, entry) = settings_index::entry("primary-color")?;
    Some(match store.settings.primary() {
        Some(chosen) => settings_appearance_primary_yours(&chosen.to_hex()).into(),
        None if !store.settings.uses_theme_primary() => settings_appearance_primary_pick(&base).into(),
        None => settings_appearance_primary_theme(&base, entry.description()).into(),
    })
}

/// While the theme's own primary is switched off: the theme's ANSI colors, a color chosen
/// elsewhere, and the "+" that picks any color.
fn draw_swatches(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let store = cx.global::<SettingsStore>();
    if store.settings.uses_theme_primary() {
        return None;
    }
    let base = store.base_tokens();
    let chosen = store.settings.primary();
    // The theme's own colors, without the one its primary already is.
    let theme_primary = base.get("primary").map(|c| c.with_alpha(1.0));
    let colors = offered(base, &[2, 5, 1, 3, 6, 4], theme_primary, chosen);
    Some(swatch_row("primary-swatch", colors, chosen, &window.primary_picker, Settings::choose_primary, cx))
}

/// "Pick a colour below · the theme's is …" while Custom is chosen and nothing is picked yet;
/// otherwise the table's description.
fn describe_branch(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let store = cx.global::<SettingsStore>();
    if store.settings.branch_color != BranchColor::Custom || store.settings.branch_tint().is_some() {
        return None;
    }
    let base = store.base_tokens().get("git.branch").map(|c| c.to_hex()).unwrap_or_default();
    Some(settings_appearance_branch_pick(&base).into())
}

/// While Custom is chosen: the theme's ANSI colors, the one picked, and the "+" that picks any
/// color. A pick that would not read on the window is made readable, as the theme's is.
fn draw_branch_swatches(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let store = cx.global::<SettingsStore>();
    if store.settings.branch_color != BranchColor::Custom {
        return None;
    }
    let chosen = match store.settings.branch_tint() {
        Some(BranchTint::Color(color)) => Some(color),
        _ => None,
    };
    let colors = offered(store.base_tokens(), &[5, 1, 3, 2, 6, 4], None, chosen);
    Some(swatch_row(
        "branch-swatch",
        colors,
        chosen,
        &window.branch_picker,
        Settings::choose_branch_color,
        cx,
    ))
}

/// The theme's ANSI colors at `ansi` in that order, each once and without `skip`, then
/// `chosen` when it is none of them.
fn offered(base: &ThemeTokens, ansi: &[usize], skip: Option<Color>, chosen: Option<Color>) -> Vec<Color> {
    let mut colors: Vec<Color> = Vec::new();
    let candidates = ansi.iter().filter_map(|&ix| base.terminal.ansi.get(ix)).map(|c| c.with_alpha(1.0));
    for color in candidates.chain(chosen) {
        if Some(color.to_hex()) != skip.map(|c| c.to_hex())
            && !colors.iter().any(|c| c.to_hex() == color.to_hex())
        {
            colors.push(color);
        }
    }
    colors
}

/// A row of swatches, the `chosen` one marked, that `choose` a color by its hex, then
/// `picker`'s "+" for any other.
fn swatch_row(
    id: &'static str,
    colors: Vec<Color>,
    chosen: Option<Color>,
    picker: &Entity<ColorPickerState>,
    choose: fn(&mut Settings, String),
    cx: &App,
) -> AnyElement {
    let ui = ui(cx);
    div()
        .flex()
        .items_center()
        .gap_1()
        .children(colors.into_iter().enumerate().map(|(ix, color)| {
            let hex = color.to_hex();
            let picked = chosen.is_some_and(|c| c.to_hex() == hex);
            style::swatch((id, ix), ui, theme_bridge::hsla(color), picked)
                .on_click(move |_, _, cx| update_settings(cx, |s| choose(s, hex.clone())))
        }))
        .child(ColorPicker::new(picker).icon(IconName::Plus).anchor(Anchor::TopRight))
        .into_any_element()
}

/// The opacity slider at what the settings ask for, switched off while Reduce Transparency or
/// Increase Contrast makes every window opaque whatever it says.
fn draw_opacity(w: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let store = cx.global::<SettingsStore>();
    let reduced = store.system().opaque();
    let value = format!("{:.0} %", store.asked_opacity() * 100.0);
    let ui = ui(cx);
    // The value goes quiet with the slider.
    let shown = if reduced { UiColors { text: ui.faint, ..ui } } else { ui };
    let slider = Slider::new(&w.opacity_slider).disabled(reduced);
    style::slider_control(shown, slider, value, &["30", "65", "100"], false).into_any_element()
}

/// Why the opacity slider is off: the system's Reduce Transparency or Increase
/// Contrast.
fn reduced_transparency_note(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let system = cx.global::<SettingsStore>().system();
    let note = match (system.reduce_transparency, system.increase_contrast) {
        (true, _) => text(SETTINGS_APPEARANCE_OPAQUE_REDUCE_TRANSPARENCY),
        (false, true) => text(SETTINGS_APPEARANCE_OPAQUE_INCREASE_CONTRAST),
        (false, false) => return None,
    };
    Some(style::banner(ui(cx), style::BannerKind::Info, false, note).into_any_element())
}

/// Below 95 % without the blur the desktop shows sharp behind the text, and no text colour
/// reads on whatever it is.
fn blur_off_warning(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let s = settings(cx);
    let look = Translucency { opacity: s.opacity, blur: s.blur };
    look.unsafe_without_blur().then(|| {
        style::banner(ui(cx), style::BannerKind::Warning, false, text(SETTINGS_APPEARANCE_BLUR_OFF_WARNING))
            .child(
                Button::new("blur-on")
                    .ghost()
                    .small()
                    .label(text(SETTINGS_APPEARANCE_TURN_ON_BLUR))
                    .on_click(|_, _, cx| update_settings(cx, |s| s.blur = true)),
            )
            .into_any_element()
    })
}

/// The settings and Studio windows in one row of checkboxes.
fn draw_translucent(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let (ui, s) = (ui(cx), settings(cx));
    div()
        .flex()
        .items_center()
        .gap_4()
        .child(
            style::checkbox("translucent-settings", ui, s.settings_translucent)
                .in_form(text(SETTINGS_APPEARANCE_SETTINGS))
                .on_change(|checked, _, cx| update_settings(cx, |s| s.settings_translucent = checked)),
        )
        .child(
            style::checkbox("translucent-studio", ui, s.studio_translucent)
                .in_form(text(SETTINGS_APPEARANCE_THEME_STUDIO))
                .on_change(|checked, _, cx| update_settings(cx, |s| s.studio_translucent = checked)),
        )
        .into_any_element()
}

fn draw_icon_select(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let current = settings(cx).icon_theme.clone();
    let choices = crate::icons::available().into_iter().map(|name| Choice::new(name.clone(), name)).collect();
    window.select(
        "icon-theme",
        div().child(current.clone()).into_any_element(),
        style::RECORDER_W,
        choices,
        current,
        |s, name| s.icon_theme = name.to_string(),
        cx,
    )
}

/// The project's theme notice, the filter and the cards, and the ways to more themes.
fn draw_theme(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let project = ProjectSource::current(&window.front, cx).filter(|project| project.theme);
    div()
        .flex()
        .flex_col()
        .gap_3()
        .pt_3()
        .pb_3()
        .rule_top(ui)
        .children(project.map(|project| {
            let root = project.root.clone();
            style::banner(
                ui,
                style::BannerKind::Info,
                false,
                settings_appearance_project_theme_note(&project.name),
            )
            .child(
                Button::new("project-theme-settings")
                    .ghost()
                    .small()
                    .label(text(SETTINGS_APPEARANCE_PROJECT_SETTINGS))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.projects.select(root.clone());
                        this.show_section(Section::Projects, cx);
                    })),
            )
        }))
        .child(window.render_theme_picker(cx))
        .into_any_element()
}
