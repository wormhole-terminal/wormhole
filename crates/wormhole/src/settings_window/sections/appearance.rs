//! Appearance: the live preview, the mode and the theme cards, the primary and the branch
//! color as swatches, the window's opacity, blur, Liquid Glass (a beta, from macOS 26) and status bar,
//! the title row's parts (`title_row`), the file icons. Under Reduce
//! Transparency the opacity slider is off and says why; with the blur off below 95 % the
//! blur's row warns.

use gpui_kit::assets::IconName;
use gpui_kit::component::slider::Slider;
use gpui_kit::*;
use wormhole_core::config::ThemeMode;
use wormhole_core::config::settings_index::{self, SectionId as Section};
use wormhole_core::theme::tokens::Translucency;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{color_rows, open_button, stage, title_row};
use crate::select::{self, MenuOptions};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, UiColors, settings};
use crate::settings_window::registry::{MenuOption, ProjectSource, Setting, row_label, toggle};
use crate::settings_window::{SettingsWindow, open_folder, ui};
use crate::style::{self, Edges};

const MODES: &[MenuOption] = &[
    MenuOption::text(SETTINGS_APPEARANCE_MATCH_SYSTEM),
    MenuOption::text(SETTINGS_APPEARANCE_DARK),
    MenuOption::text(SETTINGS_APPEARANCE_LIGHT),
];

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("appearance-preview", stage::draw_stage),
    Setting::menu(
        "theme-mode",
        MODES,
        |s| match s.theme_mode {
            ThemeMode::System => 0,
            ThemeMode::Dark => 1,
            ThemeMode::Light => 2,
        },
        |s, ix| {
            s.theme_mode = match ix {
                0 => ThemeMode::System,
                1 => ThemeMode::Dark,
                _ => ThemeMode::Light,
            }
        },
    ),
    Setting::block("theme", draw_theme),
    Setting::custom("primary-color", color_rows::draw_primary)
        .describe(describe_primary)
        .default_words(color_rows::theme_words),
    Setting::custom("branch-color", color_rows::draw_branch).default_words(color_rows::theme_words),
    Setting::custom("opacity", draw_opacity)
        .below(reduced_transparency_note)
        .default_words(|| format!("{:.0} %", Settings::default().opacity * 100.0).into()),
    toggle!("blur", blur).below(blur_off_warning),
    toggle!("liquid-glass", liquid_glass)
        .chip(|_, _| Some(text(SETTINGS_APPEARANCE_LIQUID_GLASS_BETA).into()))
        .below(glass_held_note),
    Setting::custom("translucent", draw_translucent).default_words(default_translucent),
    toggle!("column-dividers", column_dividers),
    toggle!("status-bar", status_bar),
    toggle!("icon-animations", icon_animations),
    Setting::custom("title-row", title_row::draw_control)
        .below(title_row::draw_parts)
        .default_words(title_row::default_parts),
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

/// The two windows the opacity and blur may reach, by default: neither, as the checkboxes show.
fn default_translucent() -> SharedString {
    let defaults = Settings::default();
    text(if defaults.settings_translucent || defaults.studio_translucent {
        SETTINGS_CHANGED_ON
    } else {
        SETTINGS_CHANGED_OFF
    })
    .into()
}

/// "The theme's · #7aa2f7. Off: …": the primary in use, the theme's or the one chosen.
fn describe_primary(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let store = cx.global::<SettingsStore>();
    let base = store.base_tokens().get("primary").map(|c| c.to_hex()).unwrap_or_default();
    let (_, _, entry) = settings_index::entry("primary-color")?;
    Some(match store.settings.primary() {
        Some(chosen) => settings_appearance_primary_yours(&chosen.to_hex()).into(),
        None => settings_appearance_primary_theme(&base, entry.description()).into(),
    })
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
    Some(style::notice(ui(cx), style::NoticeKind::Info, note).into_any_element())
}

/// Below 95 % without the blur the desktop shows sharp behind the text, and no text colour
/// reads on whatever it is.
fn blur_off_warning(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let s = settings(cx);
    let look = Translucency { opacity: s.opacity, blur: s.blur };
    look.unsafe_without_blur().then(|| {
        style::notice(ui(cx), style::NoticeKind::Warning, text(SETTINGS_APPEARANCE_BLUR_OFF_WARNING))
            .child(
                style::notice_button("blur-on", ui(cx), style::NoticeKind::Warning, cx)
                    .label(text(SETTINGS_APPEARANCE_TURN_ON_BLUR))
                    .on_click(|_, _, cx| update_settings(cx, |s| s.blur = true)),
            )
            .into_any_element()
    })
}

/// Under Reduce Transparency or Increase Contrast the switch can be on and still show no glass: say why.
fn glass_held_note(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    (settings(cx).liquid_glass && cx.global::<SettingsStore>().system().opaque()).then(|| {
        style::notice(ui(cx), style::NoticeKind::Info, text(SETTINGS_APPEARANCE_GLASS_HELD_BY_ACCESSIBILITY))
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

fn draw_icon_select(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let current = settings(cx).icon_theme.clone();
    select::menu_select("icon-theme", ui(cx), row_label("icon-theme"), current.clone(), move |_| {
        let mut names = crate::icons::available();
        // A theme that is not installed any more is shown as written, and listed, until another is chosen.
        let at = names.iter().position(|name| *name == current).unwrap_or_else(|| {
            names.push(current.clone());
            names.len() - 1
        });
        let picked = names.clone();
        MenuOptions::words(names.into_iter().map(Into::into), Some(at), move |ix, _, cx| {
            if let Some(name) = picked.get(ix) {
                update_settings(cx, |s| s.icon_theme = name.clone());
            }
        })
    })
    .into_any_element()
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
            style::notice(ui, style::NoticeKind::Info, settings_appearance_project_theme_note(&project.name))
                .child(
                    style::notice_button("project-theme-settings", ui, style::NoticeKind::Info, cx)
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
