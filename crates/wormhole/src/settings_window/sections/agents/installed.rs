//! The agents on this Mac, one tile row each, and their profiles' logins.

use std::path::Path;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::Agent;
use wormhole_core::agents::integrations::{self, Spec, State};
use wormhole_core::config::settings_index::SectionId as Section;

use super::snapshot::{AgentRow, LoginRow};
use crate::settings::UiColors;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

/// The installed agents, one row each.
pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else { return div().into_any_element() };
    let rows: Vec<AnyElement> =
        snapshot.installed().map(|row| agent_row(ui, row, snapshot.home(), cx)).collect();
    let none = rows.is_empty().then(|| {
        style::empty_list(
            ui,
            style::EmptyList {
                icon: IconName::SquareTerminal,
                title: text(SETTINGS_AGENTS_NONE_TITLE).into(),
                hint: text(SETTINGS_AGENTS_NO_CODING_AGENT_ON_THIS_MACS).into(),
                actions: Vec::new(),
            },
        )
    });
    div().flex().flex_col().children(rows).children(none).into_any_element()
}

/// One installed agent as a tile row: its name with the hooks' chip, where the hooks are (and Reveal), the one
/// action that fits; and under it what needs doing, and each profile's login.
fn agent_row(ui: UiColors, row: &AgentRow, home: &Path, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let agent = row.agent;
    let (chip_kind, chip) = match (row.spec, row.hooks()) {
        (Some(_), Some(State::Installed)) => (style::ChipKind::Good, text(SETTINGS_AGENTS_STATUS_HOOKS_ON)),
        (Some(_), Some(State::Partial)) => (style::ChipKind::Warn, text(SETTINGS_AGENTS_HOOKS_INCOMPLETE)),
        _ => (style::ChipKind::Quiet, text(SETTINGS_AGENTS_READING_OUTPUT)),
    };
    let path = row.spec.map(|spec| (spec.display_path(home), spec.path(home)));
    let detail = match row.hooks() {
        Some(State::Installed) => text(SETTINGS_AGENTS_INSTALLED_HOOKS_IN),
        Some(State::Partial) => text(SETTINGS_AGENTS_INSTALLED_HOOKS_INCOMPLETE_IN),
        Some(State::NotInstalled) => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_HOOKS_GO),
        None => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_NO_HOOK),
    };
    let hint = match &path {
        Some((shown, _)) => format!("{detail} {shown}"),
        None => detail.to_string(),
    };
    let mut parts = style::TileRowParts::new(style::RowTile::Agent(agent.id), agent.name);
    parts.chip = Some((chip_kind, chip.into()));
    parts.hint = Some(hint.into());
    if let Some((_, full)) = path {
        parts.controls.push(
            style::icon_button(
                SharedString::from(format!("agent-reveal-{:?}", agent.id)),
                ui,
                IconName::ExternalLink,
                text(SETTINGS_AGENTS_REVEAL_IN_FINDER),
            )
            .in_row()
            .on_click(move |_, _, cx| reveal_file(&full, cx))
            .into_any_element(),
        );
    }
    if let (Some(spec), Some(state)) = (row.spec, row.hooks()) {
        let remove = state == State::Installed;
        let label = match state {
            State::Installed => text(SETTINGS_AGENTS_REMOVE_HOOKS),
            State::Partial => text(SETTINGS_AGENTS_REPAIR_HOOKS),
            State::NotInstalled => text(SETTINGS_AGENTS_INSTALL_HOOKS),
        };
        parts.controls.push(
            style::form_button(SharedString::from(format!("agent-hooks-{:?}", agent.id)))
                .when(remove, |b| b.outline())
                .when(!remove, |b| b.primary())
                .label(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let home = integrations::home();
                    let result = if remove { spec.uninstall(&home) } else { spec.install(&home) };
                    if let Err(err) = &result {
                        tracing::warn!("{} hooks: {err}", agent.name);
                    }
                    this.notice = result.err().map(|e| {
                        (Section::Agents, settings_agents_hooks_failed(agent.name, &e.to_string()).into())
                    });
                    // The files changed: read them again.
                    this.load_agents(cx);
                    cx.notify();
                }))
                .into_any_element(),
        );
    }
    let problem = match &row.state {
        Some(Err(err)) => Some(style::notice(ui, style::NoticeKind::Danger, err.clone())),
        Some(Ok(State::Partial)) => {
            Some(style::notice(ui, style::NoticeKind::Warning, settings_agents_hooks_partial(agent.name)))
        }
        _ => None,
    };
    let note = row.spec.and_then(|spec| spec.note).filter(|_| row.hooks() != Some(State::NotInstalled));
    div()
        .flex()
        .flex_col()
        .child(style::tile_row(SharedString::from(format!("agent-{:?}", agent.id)), ui, parts))
        .children(note.map(|note| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .pb_2()
                .pl(px(style::ROW_TILE + 12.0))
                .text_caption()
                .text_color(ui.muted)
                .child(style::icon(IconName::Info))
                .child(note)
        }))
        .children(problem.map(|banner| div().pb_2().pl(px(style::ROW_TILE + 12.0)).child(banner)))
        .children(row.spec.map(|spec| {
            div().flex().flex_col().children(
                row.logins.iter().enumerate().map(|(ix, login)| login_row(ui, agent, spec, ix, login, cx)),
            )
        }))
        .into_any_element()
}

/// A profile's login of an agent: its folder, the hooks' state there and the button for it, as the
/// agent's own row has for the default location.
fn login_row(
    ui: UiColors,
    agent: &'static Agent,
    spec: &'static Spec,
    ix: usize,
    login: &LoginRow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let (code_font, _) = crate::fonts::code_font(cx);
    let state = login.state.as_ref().ok().copied();
    let dir = login.dir.clone();
    let shown = crate::settings_window::with_tilde(&dir);
    let (label, remove) = match state {
        Some(State::Installed) => (text(SETTINGS_AGENTS_REMOVE_HOOKS), true),
        Some(State::Partial) => (text(SETTINGS_AGENTS_REPAIR_HOOKS), false),
        _ => (text(SETTINGS_AGENTS_INSTALL_HOOKS), false),
    };
    let badge = match state {
        Some(State::Installed) => text(SETTINGS_AGENTS_STATUS_HOOKS_ON),
        Some(State::Partial) => text(SETTINGS_AGENTS_HOOKS_INCOMPLETE),
        _ => text(SETTINGS_AGENTS_NO_HOOKS),
    };
    let id = format!("agent-login-{:?}-{ix}", agent.id);
    div()
        .flex()
        .items_center()
        .gap_2()
        .pl(px(style::ROW_TILE + 12.0))
        .py_1()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1()
                .text_caption()
                .text_color(ui.muted)
                .child(settings_agents_login_state(&login.profile, badge))
                .child(
                    div()
                        .px_1()
                        .rounded(ui.radius_chip)
                        .bg(ui.hover)
                        .font_family(code_font)
                        .truncate()
                        .child(shown),
                ),
        )
        .child(
            style::form_button(SharedString::from(id))
                .when(remove, |b| b.outline())
                .when(!remove, |b| b.primary())
                .label(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let result = if remove { spec.uninstall_in(&dir) } else { spec.install_in(&dir) };
                    if let Err(err) = &result {
                        tracing::warn!("{} hooks in {}: {err}", agent.name, dir.display());
                    }
                    this.notice = result.err().map(|e| {
                        (Section::Agents, settings_agents_hooks_failed(agent.name, &e.to_string()).into())
                    });
                    this.load_agents(cx);
                    cx.notify();
                })),
        )
        .into_any_element()
}

/// Select a file in the Finder.
// GPUI's test platform panics on `reveal_path` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a view
// test that clicks this needs a stand-in first.
fn reveal_file(path: &Path, cx: &App) {
    cx.reveal_path(path);
}
