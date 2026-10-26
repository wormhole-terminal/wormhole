//! Coding agents: how many there are and how they report, a pointer to Notifications,
//! one row per installed agent with its hooks' state and the one action that fits (Install,
//! Repair, Remove), the agents that aren't on PATH folded into one row, and how status works.
//!
//! Hooks are written only by a row's own button, never by opening the page.

use std::path::{Path, PathBuf};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::integrations::{self, Spec, State};
use wormhole_core::agents::{self, AGENTS, Agent};
use wormhole_core::config::profile::Profile;
use wormhole_core::config::settings_index::SectionId as Section;

use super::cli_tool;
use crate::app_state::app;
use crate::settings::UiColors;
use crate::settings_window::registry::{Setting, toggle};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("agents-summary", draw_summary),
    Setting::custom("agents-notifications", |_, cx| {
        let ui = ui(cx);
        style::link_button("open-notifications", ui, text(SETTINGS_AGENTS_OPEN_NOTIFICATIONS), true)
            .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::Notifications, cx)))
            .into_any_element()
    }),
    Setting::block("agents-installed", draw_installed),
    Setting::block("agents-other", draw_others),
    Setting::block("agents-how", draw_how),
    cli_tool::SETTING,
    toggle!("control-write", control_write),
];

/// What the Agents page shows, read from disk: which agents are installed and what state
/// their hooks are in. That is a few hundred `stat`s (PATH) and a parse of each agent's
/// config, so it is read when the window opens and after a button changed something, on the
/// background executor, and not on every frame the window draws.
pub(crate) struct AgentsSnapshot {
    home: PathBuf,
    rows: Vec<AgentRow>,
}

pub(crate) struct AgentRow {
    pub(crate) agent: &'static Agent,
    pub(crate) installed: bool,
    /// How hooks are written for this agent, if they can be.
    pub(crate) spec: Option<&'static Spec>,
    /// `None` when there is no spec.
    pub(crate) state: Option<Result<State, String>>,
    /// The profiles' login folders for this agent: a second location for the hooks each.
    pub(crate) logins: Vec<LoginRow>,
}

/// Where a profile's login of an agent keeps its config, and the state of the hooks there.
pub(crate) struct LoginRow {
    profile: String,
    dir: PathBuf,
    state: Result<State, String>,
}

impl AgentRow {
    fn hooks(&self) -> Option<State> {
        self.state.as_ref().and_then(|state| state.as_ref().ok()).copied()
    }
}

impl AgentsSnapshot {
    /// Read every agent's state under `home`, installed agents first (the registry's order
    /// otherwise).
    pub(crate) fn load(home: &Path, profiles: &[Profile]) -> Self {
        let user_home = crate::settings_window::home();
        let mut rows: Vec<AgentRow> = AGENTS
            .iter()
            .map(|agent| {
                let spec = integrations::spec(agent.id);
                AgentRow {
                    agent,
                    installed: agents::installed(agent),
                    spec,
                    state: spec.map(|spec| spec.state(home)),
                    logins: spec
                        .map(|spec| {
                            profiles
                                .iter()
                                .flat_map(|p| {
                                    p.login_dirs(user_home.as_deref())
                                        .into_iter()
                                        .filter(|(id, _)| *id == agent.id)
                                        .map(|(_, dir)| (p.name.clone(), dir))
                                })
                                .map(|(profile, dir)| LoginRow { state: spec.state_in(&dir), profile, dir })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect();
        rows.sort_by_key(|row| !row.installed);
        Self { home: home.to_path_buf(), rows }
    }

    pub(crate) fn installed(&self) -> impl Iterator<Item = &AgentRow> {
        self.rows.iter().filter(|row| row.installed)
    }

    fn others(&self) -> impl Iterator<Item = &AgentRow> {
        self.rows.iter().filter(|row| !row.installed)
    }

    /// Installed agents whose hooks are only partly there: the nav marks the page for them.
    pub(crate) fn needs_repair(&self) -> usize {
        self.installed()
            .filter(|row| {
                row.hooks() == Some(State::Partial)
                    || row.logins.iter().any(|login| login.state == Ok(State::Partial))
            })
            .count()
    }

    pub(crate) fn home(&self) -> &Path {
        &self.home
    }
}

impl SettingsWindow {
    /// Read the agents again in the background; what is shown stays until the new list
    /// arrives.
    pub(in crate::settings_window) fn load_agents(&mut self, cx: &mut Context<Self>) {
        let profiles = app(cx).profiles.profiles().to_vec();
        self.agents_task = Some(cx.spawn(async move |this, cx| {
            let snapshot = cx
                .background_executor()
                .spawn(async move { AgentsSnapshot::load(&integrations::home(), &profiles) })
                .await;
            this.update(cx, |this, cx| {
                this.agents = Some(snapshot);
                this.agents_task = None;
                cx.notify();
            })
            .ok();
        }));
    }
}

/// Four counts across the column: installed, reporting through hooks, needing repair, and the
/// ones Wormhole knows that aren't on PATH.
fn draw_summary(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else {
        return div()
            .py_3()
            .text_body()
            .text_color(ui.muted)
            .child(text(SETTINGS_AGENTS_LOOKING_FOR_AGENTS))
            .into_any_element();
    };
    let installed = snapshot.installed().count();
    let hooked = snapshot.installed().filter(|row| row.hooks() == Some(State::Installed)).count();
    let repair = snapshot.needs_repair();
    let others = snapshot.others().count();
    let cell = |n: usize, label: &'static str, color: Hsla, first: bool| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .px_4()
            .py_2()
            .when(!first, |d| d.rule_left(ui))
            .child(
                div()
                    .text_title()
                    .font_weight(FontWeight::SEMIBOLD)
                    .tabular_nums()
                    .text_color(color)
                    .child(n.to_string()),
            )
            .child(div().text_caption().text_color(ui.muted).child(label))
    };
    div()
        .flex()
        .rule_top(ui)
        .rule_bottom(ui)
        .child(cell(installed, "installed", ui.text, true))
        .child(cell(
            hooked,
            text(SETTINGS_AGENTS_REPORTING_THROUGH_HOOKS),
            if hooked > 0 { ui.success_text } else { ui.text },
            false,
        ))
        .child(cell(
            repair,
            text(SETTINGS_AGENTS_NEEDS_REPAIR),
            if repair > 0 { ui.warning_text } else { ui.text },
            false,
        ))
        .child(cell(others, text(SETTINGS_AGENTS_RECOGNISED_NOT_ON_PATH), ui.muted, false))
        .into_any_element()
}

/// The installed agents, one row each.
fn draw_installed(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else { return div().into_any_element() };
    let rows: Vec<AnyElement> =
        snapshot.installed().map(|row| agent_row(ui, row, snapshot.home(), cx)).collect();
    let none = rows.is_empty().then(|| {
        div()
            .py_3()
            .rule_top(ui)
            .text_body()
            .text_color(ui.muted)
            .child(text(SETTINGS_AGENTS_NO_CODING_AGENT_ON_THIS_MACS))
    });
    div().flex().flex_col().children(rows).children(none).into_any_element()
}

/// An agent's logo in a square, or its monogram (CX, AI…) for an agent without a bundled logo.
fn mark(ui: UiColors, agent: &Agent) -> AnyElement {
    let tile = div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(style::CONTROL_H))
        .rounded(ui.radius)
        .bg(ui.hover);
    match agent.icon() {
        Some(logo) => tile.child(style::agent_logo(Some(logo), agent.monogram, style::GLYPH, ui.text)),
        None => tile.text_caption().font_weight(FontWeight::BOLD).text_color(ui.text).child(agent.monogram),
    }
    .into_any_element()
}

/// One installed agent: its mark and name with the hooks' badge, where the hooks are
/// (Reveal), the one action that fits, and what needs doing under it.
fn agent_row(ui: UiColors, row: &AgentRow, home: &Path, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let agent = row.agent;
    let (code_font, _) = crate::fonts::code_font(cx);
    let (badge, badge_color) = match (row.spec, row.hooks()) {
        (Some(_), Some(State::Installed)) => (text(SETTINGS_AGENTS_STATUS_HOOKS_ON), ui.success_text),
        (Some(_), Some(State::Partial)) => (text(SETTINGS_AGENTS_HOOKS_INCOMPLETE), ui.warning_text),
        _ => (text(SETTINGS_AGENTS_READING_OUTPUT), ui.muted),
    };
    let path = row.spec.map(|spec| (spec.display_path(home), spec.path(home)));
    let detail = match row.hooks() {
        Some(State::Installed) => text(SETTINGS_AGENTS_INSTALLED_HOOKS_IN),
        Some(State::Partial) => text(SETTINGS_AGENTS_INSTALLED_HOOKS_INCOMPLETE_IN),
        Some(State::NotInstalled) => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_HOOKS_GO),
        None => text(SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_NO_HOOK),
    };
    let action = match (row.spec, row.hooks()) {
        (Some(spec), Some(state)) => {
            let remove = state == State::Installed;
            let label = match state {
                State::Installed => text(SETTINGS_AGENTS_REMOVE_HOOKS),
                State::Partial => text(SETTINGS_AGENTS_REPAIR_HOOKS),
                State::NotInstalled => text(SETTINGS_AGENTS_INSTALL_HOOKS),
            };
            Some(
                Button::new(SharedString::from(format!("agent-hooks-{:?}", agent.id)))
                    .small()
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
            )
        }
        _ => None,
    };
    let problem = match &row.state {
        Some(Err(err)) => Some(style::banner(ui, style::BannerKind::Danger, false, err.clone())),
        Some(Ok(State::Partial)) => Some(style::banner(
            ui,
            style::BannerKind::Warning,
            false,
            settings_agents_hooks_partial(agent.name),
        )),
        _ => None,
    };
    let note = row.spec.and_then(|spec| spec.note).filter(|_| row.hooks() != Some(State::NotInstalled));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .py_2()
        .rule_top(ui)
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(mark(ui, agent))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_body().font_weight(FontWeight::MEDIUM).child(agent.name))
                                .child(style::status_badge(ui, badge_color, badge)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .min_w_0()
                                .text_caption()
                                .text_color(ui.muted)
                                .child(detail)
                                .children(path.map(|(shown, full)| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .px_1()
                                                .rounded(ui.radius_chip)
                                                .bg(ui.hover)
                                                .font_family(code_font.clone())
                                                .truncate()
                                                .child(shown),
                                        )
                                        .child(
                                            style::icon_button(
                                                SharedString::from(format!("agent-reveal-{:?}", agent.id)),
                                                ui,
                                                IconName::ExternalLink,
                                                text(SETTINGS_AGENTS_REVEAL_IN_FINDER),
                                            )
                                            .in_row()
                                            .on_click(move |_, _, cx| reveal_file(&full, cx)),
                                        )
                                })),
                        ),
                )
                .children(action),
        )
        .children(note.map(|note| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .pl(px(style::CONTROL_H + 12.0))
                .text_caption()
                .text_color(ui.muted)
                .child(style::icon(IconName::Info))
                .child(note)
        }))
        .children(problem.map(|banner| div().pl(px(style::CONTROL_H + 12.0)).child(banner)))
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
        .pl(px(style::CONTROL_H + 12.0))
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
            Button::new(SharedString::from(id))
                .small()
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

/// The agents that aren't on PATH, folded into one row that lists them when opened.
fn draw_others(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else { return div().into_any_element() };
    let others: Vec<&AgentRow> = snapshot.others().collect();
    if others.is_empty() {
        return div().into_any_element();
    }
    let names = others.iter().map(|row| row.agent.name).collect::<Vec<_>>().join(" · ");
    let open = window.others_open;
    let head = div()
        .id("agents-others")
        .flex()
        .items_center()
        .gap_2()
        .py_2()
        .rule_top(ui)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.hover))
        .on_click(cx.listener(|this, _, _, cx| {
            this.others_open = !this.others_open;
            cx.notify();
        }))
        .relative()
        .child(hang(style::lead(style::disclosure(ui, !open))).bottom_0().flex().items_center())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_body()
                        .font_weight(FontWeight::MEDIUM)
                        .child(settings_agents_others_head(whole(others.len()))),
                )
                .when(!open, |d| d.child(div().text_caption().text_color(ui.muted).child(names))),
        )
        .child(
            div().flex_none().text_caption().text_color(ui.faint).child(text(SETTINGS_AGENTS_NOT_ON_PATH)),
        );
    div()
        .flex()
        .flex_col()
        .child(head)
        .when(open, |d| {
            d.children(others.into_iter().map(|row| {
                let hooks = if row.spec.is_some() {
                    text(SETTINGS_AGENTS_STATUS_HOOKS_AVAILABLE)
                } else {
                    text(SETTINGS_AGENTS_STATUS_FROM_OUTPUT)
                };
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_1()
                    .child(mark(ui, row.agent))
                    .child(div().flex_1().min_w_0().text_body().text_color(ui.muted).child(row.agent.name))
                    .child(div().text_caption().text_color(ui.faint).child(hooks))
            }))
        })
        .into_any_element()
}

/// How far a lead icon hangs left of its text: its width and the gap after it, so the text
/// lines up with the labels of the rows around it.
const HANG: f32 = style::ITEM_LEAD_W + 8.0;

/// A lead icon hung in the margin before its text (see [`HANG`]), at the top of its
/// `.relative()` parent. Placed outside the layout: a negative margin would measure the text
/// around it again in every frame.
fn hang(lead: Div) -> Div {
    div().absolute().top_0().left(px(-HANG)).child(lead)
}

/// With hooks and without, side by side, and the promise about the files.
fn draw_how(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let column = |icon: IconName, color: Hsla, title: &'static str, text: &'static str| {
        div()
            .flex_1()
            .min_w_0()
            .relative()
            .flex()
            .gap_2()
            .child(hang(style::lead(style::icon(icon).text_color(color))))
            .child(
                // Its own width, so the words wrap in the column rather than run into the next one.
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().text_body().font_weight(FontWeight::MEDIUM).child(title))
                    .child(div().text_caption().text_color(ui.muted).child(text)),
            )
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .py_3()
        .child(
            div()
                .flex()
                // Room for the second column's hanging icon, and as much again before it.
                .gap(px(2.0 * HANG))
                .child(column(
                    IconName::Check,
                    ui.success,
                    text(SETTINGS_AGENTS_WITH_STATUS_HOOKS),
                    text(SETTINGS_AGENTS_THE_AGENT_TELLS_WORMHOLE_THE_MOMENT),
                ))
                .child(column(
                    IconName::SquareTerminal,
                    ui.muted,
                    text(SETTINGS_AGENTS_WITHOUT),
                    text(SETTINGS_AGENTS_STATUS_IS_READ_FROM_THE_TERMINAL),
                )),
        )
        .child(
            div()
                .text_caption()
                .text_color(ui.muted)
                .child(text(SETTINGS_AGENTS_HOOKS_DO_NOTHING_OUTSIDE_WORMHOLE_ONLY)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::integrations::{self, State};
    use wormhole_core::agents::{AGENTS, AgentId};

    use super::AgentsSnapshot;

    /// A scratch home directory, removed at the end of the test.
    struct Home(std::path::PathBuf);

    impl Home {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("wormhole-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch home");
            Self(dir)
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn state_of(snapshot: &AgentsSnapshot, id: AgentId) -> Option<State> {
        let row = snapshot.rows.iter().find(|row| row.agent.id == id).expect("every agent has a row");
        row.state.as_ref().and_then(|state| state.as_ref().ok()).copied()
    }

    #[test]
    fn the_snapshot_lists_every_agent_with_its_hook_state() {
        let home = Home::new("agents-state");
        let before = AgentsSnapshot::load(&home.0, &[]);
        assert_eq!(before.rows.len(), AGENTS.len());
        assert_eq!(state_of(&before, AgentId::Claude), Some(State::NotInstalled));

        let spec = integrations::spec(AgentId::Claude).expect("Claude Code has hooks");
        spec.install(&home.0).expect("installs into the scratch home");
        let after = AgentsSnapshot::load(&home.0, &[]);
        assert_eq!(state_of(&after, AgentId::Claude), Some(State::Installed));
        // Another agent's files were not touched.
        assert_eq!(state_of(&after, AgentId::Codex), Some(State::NotInstalled));

        spec.uninstall(&home.0).expect("removes them again");
        assert_eq!(state_of(&AgentsSnapshot::load(&home.0, &[]), AgentId::Claude), Some(State::NotInstalled));
    }

    #[test]
    fn installed_agents_come_first_and_the_registry_order_holds_otherwise() {
        let home = Home::new("agents-order");
        let snapshot = AgentsSnapshot::load(&home.0, &[]);
        let flags: Vec<bool> = snapshot.rows.iter().map(|row| row.installed).collect();
        let first_missing = flags.iter().position(|installed| !installed).unwrap_or(flags.len());
        assert!(flags[first_missing..].iter().all(|installed| !installed), "{flags:?}");
        // Within each group, the registry order.
        let order = |installed: bool| -> Vec<usize> {
            snapshot
                .rows
                .iter()
                .filter(|row| row.installed == installed)
                .map(|row| AGENTS.iter().position(|a| a.id == row.agent.id).expect("in the registry"))
                .collect()
        };
        for installed in [true, false] {
            let positions = order(installed);
            assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
        }
    }
}
