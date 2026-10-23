//! Appearance: the live preview, the mode and the theme cards, the primary as
//! swatches, the window's opacity, blur and status bar, the file icons. Under Reduce
//! Transparency the opacity slider is off and says why; with the blur off below 95 % the
//! blur's row warns.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::ColorPicker;
use gpui_kit::component::slider::Slider;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, SectionId as Section};
use wormhole_core::config::{ThemeMode, TitleBorder};
use wormhole_core::theme::tokens::Translucency;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{open_button, stage};
use crate::select::Choice;
use crate::settings::update_settings;
use crate::settings::{SettingsStore, UiColors, settings};
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
    Setting::custom("opacity", draw_opacity).below(reduced_transparency_note),
    toggle!("blur", blur).below(blur_off_warning),
    Setting::custom("liquid-glass", draw_liquid_glass).below(liquid_glass_hint),
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
    toggle!("title-branch-status", title_branch_status),
    toggle!("title-agent-gauge", title_agent_gauge),
    toggle!("title-ports", title_ports),
    toggle!("title-pull-request", title_pull_request),
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
    let ui = ui(cx);
    let store = cx.global::<SettingsStore>();
    if store.settings.uses_theme_primary() {
        return None;
    }
    let base = store.base_tokens();
    let theme_primary = base.get("primary").map(|c| c.with_alpha(1.0));
    let chosen = store.settings.primary();
    // The theme's own colors, without the one its primary already is.
    let mut colors: Vec<wormhole_core::theme::tokens::Color> = Vec::new();
    for ix in [2, 5, 1, 3, 6, 4] {
        let color = base.terminal.ansi[ix].with_alpha(1.0);
        if Some(color.to_hex()) != theme_primary.map(|c| c.to_hex())
            && !colors.iter().any(|c| c.to_hex() == color.to_hex())
        {
            colors.push(color);
        }
    }
    if let Some(chosen) = chosen
        && !colors.iter().any(|c| c.to_hex() == chosen.to_hex())
    {
        colors.push(chosen);
    }
    let swatches = div()
        .flex()
        .items_center()
        .gap_1()
        .children(colors.into_iter().enumerate().map(|(ix, color)| {
            let hex = color.to_hex();
            let picked = chosen.is_some_and(|c| c.to_hex() == hex);
            style::swatch(("primary-swatch", ix), ui, theme_bridge::hsla(color), picked).on_click(
                move |_, _, cx| {
                    let hex = hex.clone();
                    update_settings(cx, |s| s.choose_primary(hex))
                },
            )
        }))
        .child(ColorPicker::new(&window.primary_picker).icon(IconName::Plus).anchor(Anchor::TopRight));
    Some(swatches.into_any_element())
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

/// The Liquid Glass switch: out of reach before macOS 26, where there is no glass to turn on.
fn draw_liquid_glass(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let on = settings(cx).liquid_glass;
    style::toggle("liquid-glass", on, |on, _, cx| update_settings(cx, |s| s.liquid_glass = on))
        .disabled(!crate::platform::vibrancy::glass_available())
        .into_any_element()
}

fn liquid_glass_hint(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let note = if !crate::platform::vibrancy::glass_available() {
        text(SETTINGS_APPEARANCE_LIQUID_GLASS_NEEDS_MACOS_26)
    } else if cx.global::<SettingsStore>().system().opaque() {
        text(SETTINGS_APPEARANCE_GLASS_HELD_BY_ACCESSIBILITY)
    } else {
        return None;
    };
    Some(style::banner(ui(cx), style::BannerKind::Info, false, note).into_any_element())
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
