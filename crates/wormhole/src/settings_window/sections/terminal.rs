//! Terminal: scrollback, whether its text comes back after a restart, the minimap, Option as Meta per side, the font in use (set in
//! Fonts), the shell a new terminal runs and the profiles config.json lists, the bell and
//! whether programs may read the screen back, read or set the clipboard and control the window.

use std::path::Path;

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use wormhole_core::config::OptionAsMeta;
use wormhole_core::config::clipboard::{ClipboardRead, ClipboardWrite};
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::terminal::shells::{self, ShellChoice, ShellKind};
use wormhole_core::theme::fonts::FontSlot;

use super::open_in_editor;
use crate::open_in::OpenIn;
use crate::platform::apps;
use crate::platform::default_app;
use crate::select::Choice;
use crate::settings::update_settings;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::registry::{Segment, Setting, toggle};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

/// The scrollback sizes offered; a value set in config.json shows as it is.
const SCROLLBACK: [usize; 6] = [1_000, 5_000, 10_000, 50_000, 100_000, 200_000];

pub(super) const SETTINGS: &[Setting] = &[
    Setting::custom("scrollback", draw_scrollback),
    toggle!("restore-text", restore_terminal_text),
    toggle!("minimap", minimap),
    toggle!("split-gaps", split_gaps),
    Setting::segmented(
        "meta",
        &[Segment::text("Off"), Segment::text("Left"), Segment::text("Right"), Segment::text("Both")],
        |s| OptionAsMeta::ALL.iter().position(|o| *o == s.option_as_meta).unwrap_or(0),
        |s, ix| s.option_as_meta = OptionAsMeta::ALL.get(ix).copied().unwrap_or_default(),
    ),
    Setting::custom("terminal-font", |_, cx| {
        let ui = ui(cx);
        style::link_button("edit-in-fonts", ui, "Edit in Fonts", true)
            .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::Fonts, cx)))
            .into_any_element()
    })
    .describe(describe_font),
    Setting::custom("open-in", draw_open_in),
    Setting::custom("default-terminal", draw_default_terminal),
    Setting::custom("default-shell", draw_default_shell),
    Setting::custom("shells", |_, cx| {
        style::link_button("edit-shells", ui(cx), "Edit in config.json", false)
            .on_click(|_, _, _| open_in_editor(&SettingsStore::config_path()))
            .into_any_element()
    })
    .below(list_shells),
    Setting::custom("worktree-base", |_, cx| {
        style::link_button("edit-worktree-base", ui(cx), "Edit in config.json", false)
            .on_click(|_, _, _| open_in_editor(&SettingsStore::config_path()))
            .into_any_element()
    })
    .describe(describe_worktree_base),
    toggle!("bell-sound", bell_sound),
    toggle!("bell-visual", bell_visual),
    Setting::segmented(
        "clipboard-read",
        &[Segment::text("Ask"), Segment::text("Allow"), Segment::text("Deny")],
        |s| ClipboardRead::ALL.iter().position(|o| *o == s.clipboard_read).unwrap_or(0),
        |s, ix| s.clipboard_read = ClipboardRead::ALL.get(ix).copied().unwrap_or_default(),
    ),
    Setting::segmented(
        "clipboard-write",
        &[Segment::text("Allow"), Segment::text("Deny")],
        |s| ClipboardWrite::ALL.iter().position(|o| *o == s.clipboard_write).unwrap_or(0),
        |s, ix| s.clipboard_write = ClipboardWrite::ALL.get(ix).copied().unwrap_or_default(),
    ),
    toggle!("screen-checksum", allow_screen_checksum),
    toggle!("window-ops", allow_window_ops),
    toggle!("title-report", allow_title_report),
];

/// Default terminal: makes Wormhole the app that runs opened scripts, or says that it is.
fn draw_default_terminal(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let state = default_app::is_default();
    let (icon, label, tooltip) = match state {
        Some(true) => (
            IconName::Check,
            "Wormhole is the default",
            "To change it, pick another app for a .command file in Finder's Get Info",
        ),
        Some(false) => (IconName::SquareTerminal, "Make default", "Instead of Terminal"),
        None => (IconName::SquareTerminal, "Make default", "Only Wormhole.app can be the default terminal"),
    };
    Button::new("default-terminal")
        .outline()
        .small()
        .icon(icon)
        .label(label)
        .disabled(state != Some(false))
        .tooltip(tooltip)
        .on_click(cx.listener(|this, _, _, cx| {
            if let Err(err) = default_app::make_default() {
                tracing::warn!("making Wormhole the default terminal: {err:#}");
                this.notice =
                    Some((Section::Terminal, format!("Couldn't make Wormhole the default: {err}").into()));
            }
            cx.notify();
        }))
        .into_any_element()
}

/// "JetBrains Mono 14 pt · line height 1.30 · ligatures on".
fn describe_font(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let store = cx.global::<SettingsStore>();
    let s = &store.settings;
    let family = wormhole_core::theme::fonts::display_name(store.fonts.family(FontSlot::Terminal));
    Some(
        format!(
            "{family} {:.0} pt · line height {:.2} · ligatures {}",
            s.font_size,
            s.line_height,
            if s.font_ligatures { "on" } else { "off" }
        )
        .into(),
    )
}

/// Where worktrees go once `worktree_base` names a folder; the table's words until then.
fn describe_worktree_base(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let base = settings(cx).worktree_base.trim().to_string();
    if base.is_empty() {
        return None;
    }
    let home = crate::settings_window::home();
    let shown = wormhole_core::view::sidebar::display_path(Path::new(&base), home.as_deref());
    Some(
        format!("Under {shown}, in a folder named after the repository (worktree_base in config.json)")
            .into(),
    )
}

/// "10,000 lines".
fn lines(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (ix, c) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{grouped} lines")
}

fn draw_scrollback(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let current = settings(cx).scrollback_lines;
    let mut sizes = SCROLLBACK.to_vec();
    if !sizes.contains(&current) {
        sizes.push(current);
        sizes.sort_unstable();
    }
    let choices = sizes.into_iter().map(|n| Choice::new(n.to_string(), lines(n))).collect();
    window.select(
        "scrollback",
        div().child(lines(current)).into_any_element(),
        SCROLLBACK_W,
        choices,
        current.to_string(),
        |s, value| {
            if let Ok(n) = value.parse() {
                s.scrollback_lines = n;
            }
        },
        cx,
    )
}

/// The scrollback select: wide enough for "200,000 lines".
const SCROLLBACK_W: f32 = 160.0;

/// Open folders in: the apps on this Mac that can open a folder, found when the page opens,
/// the one the title row's button opens checked; an app that isn't in the list can be picked with
/// Choose app…. An app that is gone is shown as written, with what opens instead.
fn draw_open_in(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    OpenIn::refresh(cx);
    let ui = ui(cx);
    let apps = OpenIn::apps(cx);
    let resolved = OpenIn::resolved(cx);
    let current = resolved.app.bundle.clone();
    let mut options: Vec<Choice<String>> =
        apps.iter().map(|app| Choice::new(app.bundle.clone(), app.name.clone())).collect();
    if let Some(missing) = &resolved.missing {
        let chosen = settings(cx).open_in.trim().to_string();
        options.push(
            Choice::new(chosen, missing.clone())
                .detail("Not installed: the title row opens it in another app"),
        );
    }
    let face = div()
        .flex()
        .items_center()
        .gap_2()
        .child(open_in_icon(ui, cx, &resolved.app))
        .child(resolved.app.name.clone());
    let select = window.select(
        "open-in",
        face.into_any_element(),
        SHELL_W,
        options,
        current,
        |s, bundle| s.open_in = bundle.to_string(),
        cx,
    );
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(select)
        .child(
            style::link_button("open-in-choose", ui, "Choose app…", false)
                .on_click(cx.listener(|_, _, _, cx| choose_app(cx))),
        )
        .into_any_element()
}

/// An app's icon in the select, `style::GLYPH` square: its own, or its monogram.
fn open_in_icon(ui: UiColors, cx: &App, app: &wormhole_core::platform::open_in::Installed) -> AnyElement {
    match OpenIn::icon(cx, &app.bundle) {
        Some(image) => img(image).size(px(style::TITLE_GLYPH)).flex_none().into_any_element(),
        None => style::agent_logo(None, app.monogram.clone(), style::TITLE_GLYPH, ui.muted),
    }
}

/// Choose app…: an app from anywhere, kept by its bundle identifier. Anything that isn't an app
/// bundle is ignored.
fn choose_app(cx: &mut App) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Choose".into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = paths.await else { return };
        let Some(bundle) = paths.first().and_then(|path| apps::bundle_of(path)) else {
            return;
        };
        cx.update(|cx| update_settings(cx, |s| s.open_in = bundle));
    })
    .detach();
}

/// The Default shell select: the login shell, the profiles and the shells on this Mac by name,
/// what each runs under it. A default that names nothing any more (a profile taken out of
/// config.json) is shown as written, with what happens instead.
fn draw_default_shell(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let settings = settings(cx);
    let current = wormhole_core::terminal::shells::default_key(settings).to_string();
    let choices = shells::choices(settings);
    let face = choices.iter().find(|c| c.key == current).map_or(current.clone(), |c| c.name.clone());
    let mut options: Vec<Choice<String>> = choices.iter().map(shell_choice).collect();
    if !choices.iter().any(|c| c.key == current) {
        options.push(
            Choice::new(current.clone(), current.clone()).detail("Not found: the login shell runs instead"),
        );
    }
    window.select(
        "default-shell",
        div().child(face).into_any_element(),
        SHELL_W,
        options,
        current,
        |s, key| s.default_shell = key.to_string(),
        cx,
    )
}

/// A shell in the Default shell select: its name, and what it runs under it.
fn shell_choice(choice: &ShellChoice) -> Choice<String> {
    let detail = match choice.kind {
        ShellKind::Login => format!("Login shell · {}", choice.command_line),
        ShellKind::Profile | ShellKind::System => choice.command_line.clone(),
    };
    Choice::new(choice.key.clone(), choice.name.clone()).detail(detail)
}

/// The Default shell select: wide enough for a profile's name and, in the menu, what it runs.
const SHELL_W: f32 = 240.0;

/// The profiles under the Shells row, each with what it runs (as the menus run it: a bare name
/// through the login shell); nothing while config.json lists none.
fn list_shells(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let ui = ui(cx);
    let (code_font, _) = crate::fonts::code_font(cx);
    let profiles: Vec<ShellChoice> =
        shells::choices(settings(cx)).into_iter().filter(|c| c.kind == ShellKind::Profile).collect();
    if profiles.is_empty() {
        return None;
    }
    Some(
        div()
            .flex()
            .flex_col()
            .children(profiles.into_iter().map(|profile| {
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .h(px(style::CONTROL_H))
                    .child(
                        div()
                            .flex_none()
                            .w(px(SHELL_NAME_W))
                            .truncate()
                            .text_body()
                            .text_color(ui.text)
                            .child(profile.name),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_caption()
                            .font_family(code_font.clone())
                            .text_color(ui.muted)
                            .child(profile.command_line),
                    )
            }))
            .into_any_element(),
    )
}

/// A profile's name column in the Shells list, so the command lines line up.
const SHELL_NAME_W: f32 = 120.0;

#[cfg(test)]
mod tests {
    use super::lines;

    #[test]
    fn line_counts_read_with_thousands_separators() {
        assert_eq!(lines(1_000), "1,000 lines");
        assert_eq!(lines(10_000), "10,000 lines");
        assert_eq!(lines(200_000), "200,000 lines");
        assert_eq!(lines(100), "100 lines");
    }
}
