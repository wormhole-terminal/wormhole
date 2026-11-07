//! Terminal: scrollback, whether its text comes back after a restart, the minimap, Option as Meta per side, the font in use (set in
//! Fonts), the shell a new terminal runs and the profiles config.json lists, the bell and
//! whether programs may read the screen back, read or set the clipboard and control the window.

use std::path::Path;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::style::Button;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::*;
use wormhole_core::config::clipboard::{ClipboardRead, ClipboardWrite};
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::config::{IMAGE_MEMORY_STEPS_MIB, OptionAsMeta};
use wormhole_core::platform::open_in::{self, Installed, Resolved};
use wormhole_core::terminal::shells::{self, ShellChoice, ShellKind};
use wormhole_core::theme::fonts::FontSlot;

use super::open_in_editor;
use crate::open_in::OpenIn;
use crate::platform::{self, Item, Menu, apps, default_app};
use crate::select::{Choice, SelectTrigger};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, UiColors, settings};
use crate::settings_window::registry::{Segment, Setting, toggle};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

/// The scrollback sizes offered; a value set in config.json shows as it is.
const SCROLLBACK: [usize; 6] = [1_000, 5_000, 10_000, 50_000, 100_000, 200_000];

pub(super) const SETTINGS: &[Setting] = &[
    Setting::select(
        "scrollback",
        SCROLLBACK_W,
        |s| {
            scrollback_sizes(s.scrollback_lines)
                .into_iter()
                .map(|n| Choice::new(n.to_string(), lines(n)))
                .collect()
        },
        |s| s.scrollback_lines.to_string(),
        |s, value| {
            if let Ok(n) = value.parse() {
                s.scrollback_lines = n;
            }
        },
    ),
    toggle!("restore-text", restore_terminal_text),
    Setting::select(
        "image-memory",
        IMAGE_MEMORY_W,
        |s| {
            image_memory_steps(s.image_memory_mib)
                .into_iter()
                .map(|mib| {
                    Choice::new(mib.to_string(), settings_terminal_image_memory_size(&mib.to_string()))
                })
                .collect()
        },
        |s| s.image_memory_mib.to_string(),
        |s, value| {
            if let Ok(mib) = value.parse() {
                s.image_memory_mib = mib;
            }
        },
    ),
    toggle!("minimap", minimap),
    toggle!("never-hide-lines", never_hide_lines),
    toggle!("split-gaps", split_gaps),
    Setting::segmented(
        "meta",
        &[
            Segment::text(SETTINGS_TERMINAL_OFF),
            Segment::text(SETTINGS_TERMINAL_LEFT),
            Segment::text(SETTINGS_TERMINAL_RIGHT),
            Segment::text(SETTINGS_TERMINAL_BOTH),
        ],
        |s| OptionAsMeta::ALL.iter().position(|o| *o == s.option_as_meta).unwrap_or(0),
        |s, ix| s.option_as_meta = OptionAsMeta::ALL.get(ix).copied().unwrap_or_default(),
    ),
    Setting::custom("terminal-font", |_, cx| {
        let ui = ui(cx);
        style::link_button("edit-in-fonts", ui, text(SETTINGS_TERMINAL_EDIT_IN_FONTS), true)
            .on_click(cx.listener(|this, _, window, cx| this.navigate_to(Section::Fonts, window, cx)))
            .into_any_element()
    })
    .describe(describe_font),
    Setting::custom("open-in", draw_open_in),
    Setting::custom("default-terminal", draw_default_terminal),
    Setting::custom("default-shell", draw_default_shell),
    Setting::custom("shells", |_, cx| {
        style::link_button("edit-shells", ui(cx), text(SETTINGS_TERMINAL_EDIT_IN_CONFIG_JSON), false)
            .on_click(|_, _, _| open_in_editor(&SettingsStore::config_path()))
            .into_any_element()
    })
    .below(list_shells),
    Setting::custom("worktree-base", |_, cx| {
        style::link_button("edit-worktree-base", ui(cx), text(SETTINGS_TERMINAL_EDIT_IN_CONFIG_JSON), false)
            .on_click(|_, _, _| open_in_editor(&SettingsStore::config_path()))
            .into_any_element()
    })
    .describe(describe_worktree_base),
    toggle!("worktree-clone-build-folders", worktree_clone_build_folders),
    toggle!("bell-sound", bell_sound),
    toggle!("bell-visual", bell_visual),
    Setting::segmented(
        "clipboard-read",
        &[
            Segment::text(SETTINGS_TERMINAL_ASK),
            Segment::text(SETTINGS_TERMINAL_ALLOW),
            Segment::text(SETTINGS_TERMINAL_DENY),
        ],
        |s| ClipboardRead::ALL.iter().position(|o| *o == s.clipboard_read).unwrap_or(0),
        |s, ix| s.clipboard_read = ClipboardRead::ALL.get(ix).copied().unwrap_or_default(),
    ),
    Setting::segmented(
        "clipboard-write",
        &[Segment::text(SETTINGS_TERMINAL_ALLOW), Segment::text(SETTINGS_TERMINAL_DENY)],
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
            text(SETTINGS_TERMINAL_WORMHOLE_IS_THE_DEFAULT),
            text(SETTINGS_TERMINAL_TO_CHANGE_IT_PICK_ANOTHER_APP),
        ),
        Some(false) => (
            IconName::SquareTerminal,
            text(SETTINGS_TERMINAL_MAKE_DEFAULT),
            text(SETTINGS_TERMINAL_INSTEAD_OF_TERMINAL),
        ),
        None => (
            IconName::SquareTerminal,
            text(SETTINGS_TERMINAL_MAKE_DEFAULT),
            text(SETTINGS_TERMINAL_DEFAULT_ONLY_APP),
        ),
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
                    Some((Section::Terminal, settings_terminal_default_failed(&err.to_string()).into()));
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
    let (size, line_height) = (format!("{:.0}", s.font_size), format!("{:.2}", s.line_height));
    let summary = if s.font_ligatures {
        settings_terminal_font_summary_ligatures_on(family, &size, &line_height)
    } else {
        settings_terminal_font_summary_ligatures_off(family, &size, &line_height)
    };
    Some(summary.into())
}

/// Where worktrees go once `worktree_base` names a folder; the table's words until then.
fn describe_worktree_base(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let base = settings(cx).worktree_base.trim().to_string();
    if base.is_empty() {
        return None;
    }
    let home = crate::settings_window::home();
    let shown = wormhole_core::view::sidebar::display_path(Path::new(&base), home.as_deref());
    Some(settings_terminal_worktree_base_under(&shown).into())
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
    settings_terminal_scrollback_lines(&grouped)
}

fn scrollback_sizes(current: usize) -> Vec<usize> {
    let mut sizes = SCROLLBACK.to_vec();
    if !sizes.contains(&current) {
        sizes.push(current);
        sizes.sort_unstable();
    }
    sizes
}

/// The image memory sizes offered (core's steps); a value set in config.json shows as it is.
fn image_memory_steps(current: u32) -> Vec<u32> {
    let mut steps = IMAGE_MEMORY_STEPS_MIB.to_vec();
    if !steps.contains(&current) {
        steps.push(current);
        steps.sort_unstable();
    }
    steps
}

/// The image memory select: wide enough for "1024 MiB".
const IMAGE_MEMORY_W: f32 = 120.0;

/// The scrollback select: wide enough for "200,000 lines".
const SCROLLBACK_W: f32 = 160.0;

/// Open folders in: the apps on this Mac that can open a folder, found when Settings opens or the page is
/// shown (`OpenIn::refresh`), the one the title row's button opens checked; an app that isn't in the list can
/// be picked with Choose app…. An app that is gone is shown as written, with what opens instead.
fn draw_open_in(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let (_, resolved, options) = open_in_choices(cx);
    let face = div()
        .flex()
        .items_center()
        .gap_2()
        .child(open_in_icon(ui, cx, &resolved.app))
        .child(resolved.app.name.clone());
    let select = if open_in_has_native_menu(&resolved) {
        SelectTrigger::new("select-open-in-trigger", ui, face)
            .width(px(SHELL_W))
            .with_menu(text(SETTINGS_TERMINAL_OPEN_IN), resolved.app.name.into(), |cx| open_in_menu(cx))
            .into_any_element()
    } else {
        window.select(
            "open-in",
            face.into_any_element(),
            SHELL_W,
            options,
            resolved.app.bundle,
            |s, bundle| s.open_in = bundle.to_string(),
            cx,
        )
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(select)
        .child(
            style::link_button("open-in-choose", ui, text(SETTINGS_TERMINAL_CHOOSE_APP), false)
                .on_click(cx.listener(|_, _, _, cx| choose_app(cx))),
        )
        .into_any_element()
}

fn open_in_choices(cx: &App) -> (Vec<Installed>, Resolved, Vec<Choice<String>>) {
    let apps = OpenIn::apps(cx);
    let chosen = settings(cx).open_in.trim();
    let resolved = open_in::resolve(chosen, &apps);
    let mut options: Vec<_> =
        apps.iter().map(|app| Choice::new(app.bundle.clone(), app.name.clone())).collect();
    if let Some(missing) = &resolved.missing {
        options.push(
            Choice::new(chosen.to_string(), missing.clone())
                .detail(text(SETTINGS_TERMINAL_NOT_INSTALLED_THE_TITLE_ROW_OPENS)),
        );
    }
    (apps, resolved, options)
}

fn open_in_has_native_menu(resolved: &Resolved) -> bool {
    platform::native_menus() && (resolved.missing.is_none() || platform::native_menu_subtitles())
}

fn open_in_menu(cx: &App) -> Menu {
    let (apps, resolved, options) = open_in_choices(cx);
    options.into_iter().enumerate().fold(Menu::new(), |menu, (index, option)| {
        // The installed apps are the first options, in the same order. A missing app has no icon.
        let icon = apps.get(index).and_then(|app| OpenIn::menu_icon(cx, app));
        let item = choice_item(&option, &resolved.app.bundle).icon(icon);
        menu.item(item.on_select(move |_, cx| update_settings(cx, |s| s.open_in.clone_from(&option.value))))
    })
}

/// An app's icon in the select, `style::GLYPH` square: its own, or its monogram.
fn open_in_icon(ui: UiColors, cx: &App, app: &Installed) -> AnyElement {
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
        prompt: Some(text(SETTINGS_TERMINAL_CHOOSE).into()),
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
    let (current, options) = default_shell_choices(settings(cx));
    let face = options
        .iter()
        .find(|option| option.value == current)
        .map_or_else(|| current.clone().into(), |option| option.label().clone());
    if platform::native_menu_subtitles() {
        return SelectTrigger::new("select-default-shell-trigger", ui(cx), face.clone())
            .width(px(SHELL_W))
            .with_menu(text(SETTINGS_TERMINAL_DEFAULT_SHELL), face, |cx| default_shell_menu(cx))
            .into_any_element();
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

fn default_shell_choices(settings: &Settings) -> (String, Vec<Choice<String>>) {
    let current = shells::default_key(settings).to_string();
    let mut options: Vec<_> = shells::choices(settings).iter().map(shell_choice).collect();
    if !options.iter().any(|option| option.value == current) {
        options.push(
            Choice::new(current.clone(), current.clone())
                .detail(text(SETTINGS_TERMINAL_NOT_FOUND_THE_LOGIN_SHELL_RUNS)),
        );
    }
    (current, options)
}

fn default_shell_menu(cx: &App) -> Menu {
    let (current, options) = default_shell_choices(settings(cx));
    options.into_iter().fold(Menu::new(), |menu, option| {
        let item = choice_item(&option, &current);
        menu.item(item.on_select(move |_, cx| {
            update_settings(cx, |s| s.default_shell.clone_from(&option.value));
        }))
    })
}

fn choice_item(option: &Choice<String>, current: &str) -> Item {
    let item = Item::new(option.label().clone()).checked(option.value == current);
    match option.detail_text() {
        Some(detail) => item.detail(detail.clone()),
        None => item,
    }
}

#[cfg(feature = "devctl")]
pub(in crate::settings_window) fn rich_select_menu(id: &str, cx: &App) -> Option<Menu> {
    match id {
        "open-in" if open_in_has_native_menu(&OpenIn::resolved(cx)) => Some(open_in_menu(cx)),
        "default-shell" if platform::native_menu_subtitles() => Some(default_shell_menu(cx)),
        _ => None,
    }
}

/// A shell in the Default shell select: its name, and what it runs under it.
fn shell_choice(choice: &ShellChoice) -> Choice<String> {
    let detail = match choice.kind {
        ShellKind::Login => settings_terminal_login_shell(&choice.command_line),
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
