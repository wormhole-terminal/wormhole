//! Coding agents → Command line → Command line tool: the `wh` command for terminals that aren't
//! Wormhole's own (those have it already). Install writes a small script into `/usr/local/bin` or
//! `~/.local/bin`, Uninstall takes it out again, and a folder that isn't on the login shell's `PATH`
//! comes with the line to add. All of it is `wormhole_core::terminal::cli_tool`; this draws the row and calls it.
//!
//! The script is written only by the Install (or Update) button, never by opening the page.

use std::path::{Path, PathBuf};
use wormhole_text::{keys::*, text};

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::ipc::control::COMMAND_NAME;
use wormhole_core::terminal::cli_tool::{self, Advice, Status, Targets};

use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

pub(super) const SETTING: Setting = Setting::custom("cli-tool", draw).describe(describe).below(below);

/// What the row shows, read when the window opens and after a button was pressed.
pub(crate) struct CliToolPanel {
    status: Status,
    path: PathNote,
    /// What the last press could not do, until the next one.
    problem: Option<SharedString>,
    /// The login shell's answer, on its way.
    task: Option<Task<()>>,
}

/// Whether the installed script's folder is on the `PATH` a new terminal gets.
enum PathNote {
    /// Nothing installed, or not asked.
    None,
    Reading,
    On(SharedString),
    /// Not on it: the folder as the person would say it, and the line that puts it there.
    Off(SharedString, Advice),
    /// The shell wouldn't say.
    Unknown(SharedString),
}

impl CliToolPanel {
    pub(crate) fn read() -> Self {
        Self { status: current_status(), path: PathNote::None, problem: None, task: None }
    }
}

/// The running binary, which the script runs.
fn binary() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn current_status() -> Status {
    cli_tool::status(&Targets::from_env(), &binary())
}

/// `path` as a person says it (`~/.local/bin/wh`).
fn shown(path: &Path) -> String {
    wormhole_core::view::sidebar::display_path(path, crate::settings_window::home().as_deref())
}

impl SettingsWindow {
    /// Read where the tool stands, and, when it is installed, whether its folder is on a new
    /// terminal's `PATH`. That asks the login shell, which reads the person's startup files: in the
    /// background, and not again until a button changes something.
    pub(in crate::settings_window) fn load_cli_tool(&mut self, cx: &mut Context<Self>) {
        self.cli_tool.status = current_status();
        self.cli_tool.task = None;
        let Status::Installed { path, .. } = &self.cli_tool.status else {
            self.cli_tool.path = PathNote::None;
            cx.notify();
            return;
        };
        let Some(dir) = path.parent().map(Path::to_path_buf) else { return };
        self.cli_tool.path = PathNote::Reading;
        self.cli_tool.task = Some(cx.spawn(async move |this, cx| {
            let note = cx.background_executor().spawn(async move { path_note(&dir) }).await;
            this.update(cx, |this, cx| {
                this.cli_tool.path = note;
                this.cli_tool.task = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    pub(in crate::settings_window) fn install_cli_tool(&mut self, cx: &mut Context<Self>) {
        self.cli_tool.problem = match cli_tool::install(&Targets::from_env(), &binary()) {
            Ok(done) => {
                tracing::info!(path = %done.path.display(), changed = done.changed, "installed the command line tool");
                None
            }
            // The row already says whose `wh` is in the way.
            Err(cli_tool::Error::Foreign(_)) => None,
            Err(err) => {
                tracing::warn!("installing the command line tool: {err}");
                Some(settings_agents_cli_install_failed(COMMAND_NAME, &err.to_string()).into())
            }
        };
        self.load_cli_tool(cx);
    }

    pub(in crate::settings_window) fn uninstall_cli_tool(&mut self, cx: &mut Context<Self>) {
        self.cli_tool.problem = match cli_tool::uninstall(&Targets::from_env()) {
            Ok(removed) => {
                tracing::info!(removed = removed.len(), "removed the command line tool");
                None
            }
            Err(err) => {
                tracing::warn!("removing the command line tool: {err}");
                Some(settings_agents_cli_remove_failed(COMMAND_NAME, &err.to_string()).into())
            }
        };
        self.load_cli_tool(cx);
    }
}

/// Ask the login shell for its `PATH` and compare it with `dir`.
fn path_note(dir: &Path) -> PathNote {
    let said = shown(dir);
    let Some(path) = cli_tool::login_shell_path() else {
        return PathNote::Unknown(said.into());
    };
    if cli_tool::on_path(dir, &path) {
        return PathNote::On(said.into());
    }
    let advice = cli_tool::advice(
        dir,
        &wormhole_core::terminal::shells::login_shell(),
        crate::settings_window::home().as_deref(),
    );
    PathNote::Off(said.into(), advice)
}

/// The row's words: what it is for, and where it stands.
fn describe(window: &SettingsWindow, _: &mut Context<SettingsWindow>) -> Option<SharedString> {
    Some(
        match &window.cli_tool.status {
            Status::NotInstalled => settings_agents_cli_not_installed(COMMAND_NAME),
            Status::Installed { path, .. } => settings_agents_cli_installed_at(COMMAND_NAME, &shown(path)),
            Status::Foreign { path } => settings_agents_cli_foreign_at(COMMAND_NAME, &shown(path)),
        }
        .into(),
    )
}

/// Install, Update and Uninstall.
fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let install = |label: &'static str, cx: &mut Context<SettingsWindow>| {
        Button::new("cli-tool-install")
            .small()
            .primary()
            .label(label)
            .on_click(cx.listener(|this, _, _, cx| this.install_cli_tool(cx)))
    };
    let uninstall = |cx: &mut Context<SettingsWindow>| {
        Button::new("cli-tool-uninstall")
            .small()
            .outline()
            .label(text(SETTINGS_AGENTS_UNINSTALL))
            .on_click(cx.listener(|this, _, _, cx| this.uninstall_cli_tool(cx)))
    };
    let row = div().flex().items_center().gap_2();
    match &window.cli_tool.status {
        Status::NotInstalled => row
            .child(
                install(text(SETTINGS_AGENTS_INSTALL), cx)
                    .tooltip(settings_agents_cli_install_tooltip(COMMAND_NAME)),
            )
            .into_any_element(),
        Status::Installed { current, .. } => row
            .when(!current, |row| row.child(install(text(SETTINGS_AGENTS_UPDATE), cx)))
            .child(uninstall(cx))
            .into_any_element(),
        Status::Foreign { .. } => row
            .child(
                install(text(SETTINGS_AGENTS_INSTALL), cx)
                    .disabled(true)
                    .tooltip(settings_agents_cli_foreign_tooltip(COMMAND_NAME)),
            )
            .into_any_element(),
    }
}

/// What needs saying under the row: whose `wh` is in the way, an installed copy that runs another
/// Wormhole, and whether new terminals will find it.
fn below(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let ui = ui(cx);
    let panel = &window.cli_tool;
    let note = |color: Hsla, text: SharedString| {
        div().text_caption().text_color(color).child(text).into_any_element()
    };
    let mut lines: Vec<AnyElement> = Vec::new();
    match &panel.status {
        Status::NotInstalled => {}
        Status::Foreign { .. } => lines.push(
            style::banner(
                ui,
                style::BannerKind::Warning,
                false,
                settings_agents_cli_foreign_note(COMMAND_NAME),
            )
            .into_any_element(),
        ),
        Status::Installed { current, .. } => {
            if !current {
                lines.push(
                    style::banner(
                        ui,
                        style::BannerKind::Info,
                        false,
                        settings_agents_cli_stale_note(COMMAND_NAME),
                    )
                    .into_any_element(),
                );
            }
            match &panel.path {
                PathNote::None => {}
                PathNote::Reading => {
                    lines.push(note(ui.muted, text(SETTINGS_AGENTS_CHECKING_YOUR_SHELLS_PATH).into()))
                }
                PathNote::On(dir) => {
                    lines.push(note(ui.success_text, settings_agents_cli_path_on(dir, COMMAND_NAME).into()))
                }
                PathNote::Off(dir, advice) => {
                    let line = advice.line.clone();
                    let (code_font, _) = crate::fonts::code_font(cx);
                    lines.push(
                        style::banner(
                            ui,
                            style::BannerKind::Warning,
                            false,
                            settings_agents_cli_path_off(dir, COMMAND_NAME, &advice.file),
                        )
                        .child(
                            Button::new("cli-tool-copy-line")
                                .ghost()
                                .small()
                                .icon(IconName::Copy)
                                .label(text(SETTINGS_AGENTS_COPY_LINE))
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(line.clone()));
                                }),
                        )
                        .into_any_element(),
                    );
                    lines.push(
                        div()
                            .px_3()
                            .py_1()
                            .rounded(ui.radius)
                            .bg(ui.hover)
                            .font_family(code_font)
                            .text_caption()
                            .text_color(ui.text)
                            .child(advice.line.clone())
                            .into_any_element(),
                    );
                }
                PathNote::Unknown(dir) => {
                    lines.push(note(ui.muted, settings_agents_cli_path_unknown(COMMAND_NAME, dir).into()))
                }
            }
        }
    }
    if let Some(problem) = &panel.problem {
        lines.push(style::banner(ui, style::BannerKind::Danger, false, problem.clone()).into_any_element());
    }
    (!lines.is_empty()).then(|| div().flex().flex_col().gap_2().children(lines).into_any_element())
}
