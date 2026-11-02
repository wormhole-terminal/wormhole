//! The agents on this Mac, one tile row each, and their profiles' logins.

use std::path::{Path, PathBuf};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::Agent;
use wormhole_core::agents::integrations::{self, Spec, State};
use wormhole_core::config::settings_index::SectionId as Section;

use std::rc::Rc;

use super::snapshot::{AgentRow, LoginRow};
use crate::select::{MenuLine, MenuOptions, menu_select};
use crate::settings::UiColors;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

/// The installed agents, one row each.
pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else {
        return div()
            .py_3()
            .text_body()
            .text_color(ui.muted)
            .child(text(SETTINGS_AGENTS_LOOKING_FOR_AGENTS))
            .into_any_element();
    };
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
            cx,
        )
    });
    div().flex().flex_col().children(rows).children(none).into_any_element()
}

/// One agent on this Mac as a tile row: its name, how its status reaches Wormhole in one short line (where the
/// hooks go, and the rest of what there is to read, behind the (i)), and the control that fits the hooks'
/// state: a "Hooks on" menu (Repair, Remove, Reveal) when they are there, Add Hooks when they aren't, a
/// "Needs repair" chip and Repair Hooks when they are only partly there. Under it what needs doing, and each
/// profile's login.
fn agent_row(ui: UiColors, row: &AgentRow, home: &Path, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let agent = row.agent;
    let hooks = row.spec.zip(row.hooks());
    let path = row.spec.map(|spec| (spec.display_path(home), spec.path(home)));
    let detail = match row.hooks() {
        Some(State::Installed) => text(SETTINGS_AGENTS_INSTALLED_HOOKS_IN),
        Some(State::Partial) => text(SETTINGS_AGENTS_INSTALLED_HOOKS_INCOMPLETE_IN),
        Some(State::NotInstalled) => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_HOOKS_GO),
        None => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_NO_HOOK),
    };
    let mut more = match &path {
        Some((shown, _)) => format!("{detail} {shown}"),
        None => detail.to_string(),
    };
    if row.hooks() == Some(State::Partial) {
        more = format!("{more}. {}", settings_agents_hooks_partial(agent.name));
    }
    let mut parts = style::TileRowParts::new(style::RowTile::Agent(agent.id), agent.name);
    parts.hint = Some(
        text(match row.hooks() {
            Some(State::Installed) => SETTINGS_AGENTS_HINT_HOOKS,
            Some(State::Partial) => SETTINGS_AGENTS_HINT_PARTIAL,
            _ => SETTINGS_AGENTS_HINT_OUTPUT,
        })
        .into(),
    );
    parts.more = Some(more.into());
    if let (Some((spec, state)), Some((_, full))) = (hooks, path) {
        match state {
            State::Installed => parts.controls.push(hooks_menu(ui, agent, spec, full, cx)),
            State::Partial | State::NotInstalled => {
                if state == State::Partial {
                    parts.chip = Some((style::ChipKind::Warn, text(SETTINGS_AGENTS_NEEDS_REPAIR).into()));
                }
                // A file that isn't there has nothing to reveal.
                if row.hook_file_exists {
                    parts.controls.push(reveal_button(ui, agent, full));
                }
                let label = match state {
                    State::Partial => SETTINGS_AGENTS_REPAIR_HOOKS,
                    _ => SETTINGS_AGENTS_ADD_HOOKS,
                };
                parts.controls.push(
                    style::form_button(SharedString::from(format!("agent-hooks-{:?}", agent.id)))
                        .primary()
                        .label(text(label))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.change_hooks(agent, spec, None, false, cx);
                        }))
                        .into_any_element(),
                );
            }
        }
    }
    let problem = match &row.state {
        Some(Err(err)) => Some(style::notice(ui, style::NoticeKind::Danger, err.clone())),
        _ => None,
    };
    let note = row.spec.and_then(|spec| spec.note).filter(|_| row.hooks() != Some(State::NotInstalled));
    div()
        .flex()
        .flex_col()
        .child(style::tile_row(SharedString::from(format!("agent-{:?}", agent.id)), ui, parts, cx))
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

/// The icon button that selects the hooks' file in the Finder.
fn reveal_button(ui: UiColors, agent: &Agent, file: PathBuf) -> AnyElement {
    style::icon_button(
        SharedString::from(format!("agent-reveal-{:?}", agent.id)),
        ui,
        IconName::ExternalLink,
        text(SETTINGS_AGENTS_REVEAL_IN_FINDER),
    )
    .in_row()
    .on_click(move |_, _, cx| reveal_file(&file, cx))
    .into_any_element()
}

/// What the "Hooks on" menu does, in the order it lists them.
#[derive(Clone, Copy)]
enum HooksAction {
    Repair,
    Remove,
    Reveal,
}

/// "Hooks on" with a green dot, a menu of what can be done to hooks that are there. Every item is an action,
/// so none is checked.
fn hooks_menu(
    ui: UiColors,
    agent: &'static Agent,
    spec: &'static Spec,
    file: PathBuf,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let this = cx.weak_entity();
    let dot = div().flex_none().size(px(style::STATUS_DOT_SIZE)).rounded_full().bg(ui.success);
    menu_select(
        SharedString::from(format!("agent-hooks-{:?}", agent.id)),
        ui,
        agent.name,
        text(SETTINGS_AGENTS_HOOKS_ON),
        move |_| {
            const ACTIONS: [HooksAction; 3] = [HooksAction::Repair, HooksAction::Remove, HooksAction::Reveal];
            let words = |action: HooksAction| {
                text(match action {
                    HooksAction::Repair => SETTINGS_AGENTS_REPAIR_HOOKS,
                    HooksAction::Remove => SETTINGS_AGENTS_REMOVE_HOOKS,
                    HooksAction::Reveal => SETTINGS_AGENTS_REVEAL_HOOK_FILE,
                })
            };
            MenuOptions {
                items: ACTIONS.into_iter().map(|action| MenuLine::new(words(action)).action()).collect(),
                current: None,
                on_pick: Rc::new(move |ix, _, cx| match ACTIONS.get(ix) {
                    Some(HooksAction::Repair) => {
                        this.update(cx, |this, cx| this.change_hooks(agent, spec, None, false, cx)).ok();
                    }
                    Some(HooksAction::Remove) => {
                        this.update(cx, |this, cx| this.change_hooks(agent, spec, None, true, cx)).ok();
                    }
                    Some(HooksAction::Reveal) => reveal_file(&file, cx),
                    None => {}
                }),
                on_hover: None,
            }
        },
    )
    .leading(dot)
    .into_any_element()
}

impl SettingsWindow {
    /// Add, repair or remove an agent's hooks: in the default location, or in a profile's login folder
    /// (`login`). The files changed, so the page reads them again.
    fn change_hooks(
        &mut self,
        agent: &'static Agent,
        spec: &'static Spec,
        login: Option<&Path>,
        remove: bool,
        cx: &mut Context<Self>,
    ) {
        let result = match (login, remove) {
            (None, false) => spec.install(&integrations::home()),
            (None, true) => spec.uninstall(&integrations::home()),
            (Some(dir), false) => spec.install_in(dir),
            (Some(dir), true) => spec.uninstall_in(dir),
        };
        if let Err(err) = &result {
            tracing::warn!("{} hooks: {err}", agent.name);
        }
        self.notice = result
            .err()
            .map(|e| (Section::Agents, settings_agents_hooks_failed(agent.name, &e.to_string()).into()));
        self.load_agents(cx);
        cx.notify();
    }
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
        _ => (text(SETTINGS_AGENTS_ADD_HOOKS), false),
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
            style::form_button(SharedString::from(id)).when(!remove, |b| b.primary()).label(label).on_click(
                cx.listener(move |this, _, _, cx| {
                    this.change_hooks(agent, spec, Some(&dir), remove, cx);
                }),
            ),
        )
        .into_any_element()
}

/// Select a file in the Finder.
// GPUI's test platform panics on `reveal_path` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a view
// test that clicks this needs a stand-in first.
fn reveal_file(path: &Path, cx: &App) {
    cx.reveal_path(path);
}
