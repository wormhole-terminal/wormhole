//! Coding agents → Wormhole skill: the skill that teaches an agent to drive Wormhole through `wh`
//! (`wormhole_core::agents::skill`), one row per agent that loads skills with the one action that fits
//! (Install, Update, Uninstall), and the line that adds `wh mcp` to Claude Code, to copy.
//!
//! Nothing is written by opening the page: only a row's own button installs, updates or removes the
//! skill, in a folder of its own that is never another skill's. The page does not edit any other app's
//! MCP settings; it shows the command for the person to run.

use std::path::Path;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::skill::{self, State, Target};
use wormhole_core::agents::{self, integrations};
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::ipc::control::doctor::mcp_add_command;

use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

pub(super) const SKILLS: Setting = Setting::rows("agent-skills", draw_skills);
pub(super) const MCP: Setting = Setting::custom("agent-mcp", draw_mcp).below(mcp_command);

/// One agent's skill as the files have it, read with the rest of the page (`AgentsSnapshot::load`).
pub(crate) struct SkillRow {
    target: &'static Target,
    /// Whether the agent's command is on this Mac's `PATH`.
    found: bool,
    state: Result<State, String>,
    /// Where the skill's folder is, as a person says it.
    shown: String,
    /// The folder, read by this agent as well, that has the skill when its own has not.
    covered: Option<String>,
}

pub(crate) fn load(home: &Path) -> Vec<SkillRow> {
    skill::states(home)
        .into_iter()
        .map(|(target, state)| SkillRow {
            target,
            found: agents::installed(agents::agent(target.agent)),
            covered: skill::covered_by(target, home).filter(|_| state == Ok(State::NotInstalled)),
            state,
            shown: target.display_path(home),
        })
        .collect()
}

impl SettingsWindow {
    /// Install, update or remove the skill for `target`, and read the page again.
    fn change_skill(&mut self, target: &'static Target, remove: bool, cx: &mut Context<Self>) {
        let home = integrations::home();
        let result = if remove {
            skill::uninstall(target, &home).map(drop)
        } else {
            skill::install(target, &home).map(drop)
        };
        let name = agents::agent(target.agent).name;
        if let Err(err) = &result {
            tracing::warn!("{name} skill: {err}");
        }
        self.notice = result.err().map(|e| (Section::Agents, settings_agents_skill_failed(name, &e).into()));
        self.load_agents(cx);
        cx.notify();
    }
}

/// The rows: the agents that are on this Mac, and any that has the skill already.
fn draw_skills(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else { return div().into_any_element() };
    let rows: Vec<AnyElement> = snapshot
        .skills
        .iter()
        .filter(|row| row.found || !matches!(row.state, Ok(State::NotInstalled)))
        .map(|row| skill_row(row, cx))
        .collect();
    let none = rows.is_empty().then(|| {
        style::empty_list(
            ui,
            style::EmptyList {
                icon: IconName::SquareTerminal,
                title: text(SETTINGS_AGENTS_NONE_TITLE).into(),
                hint: text(SETTINGS_AGENTS_SKILL_NONE).into(),
                actions: Vec::new(),
            },
        )
    });
    div().flex().flex_col().children(rows).children(none).into_any_element()
}

fn skill_row(row: &SkillRow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let target = row.target;
    let agent = agents::agent(target.agent);
    let word = target.word();
    let state = row.state.as_ref().ok().copied();
    // The short line says what the skill is; where it is, and what the row does, is behind the (i).
    let (hint, detail, chip) = match state {
        Some(State::NotInstalled) if row.covered.is_some() => (
            format!("{} {}", text(SETTINGS_AGENTS_SKILL_COVERED), row.covered.as_deref().unwrap_or_default()),
            text(SETTINGS_AGENTS_SKILL_COVERED),
            None,
        ),
        Some(State::Current) => {
            (text(SETTINGS_AGENTS_SKILL_HINT_CURRENT).to_string(), text(SETTINGS_AGENTS_SKILL_CURRENT), None)
        }
        Some(State::Outdated) => (
            text(SETTINGS_AGENTS_SKILL_HINT_OUTDATED).to_string(),
            text(SETTINGS_AGENTS_SKILL_OUTDATED),
            Some((style::ChipKind::Warn, text(SETTINGS_AGENTS_SKILL_CHIP_OUTDATED))),
        ),
        Some(State::Foreign) => (
            text(SETTINGS_AGENTS_SKILL_HINT_FOREIGN).to_string(),
            text(SETTINGS_AGENTS_SKILL_FOREIGN),
            Some((style::ChipKind::Warn, text(SETTINGS_AGENTS_SKILL_CHIP_FOREIGN))),
        ),
        _ => (
            text(SETTINGS_AGENTS_SKILL_HINT_NONE).to_string(),
            text(SETTINGS_AGENTS_SKILL_NOT_INSTALLED),
            None,
        ),
    };
    let button = |id: &str, label: &'static str, primary: bool| {
        style::form_button(SharedString::from(format!("agent-skill-{word}-{id}")))
            .when(primary, |b| b.primary())
            .when(!primary, |b| b.outline())
            .label(label)
    };
    let install = |cx: &mut Context<SettingsWindow>, label, primary| {
        button("install", label, primary)
            .on_click(cx.listener(move |this, _, _, cx| this.change_skill(target, false, cx)))
            .into_any_element()
    };
    let remove = |cx: &mut Context<SettingsWindow>| {
        button("remove", text(SETTINGS_AGENTS_UNINSTALL), false)
            .on_click(cx.listener(move |this, _, _, cx| this.change_skill(target, true, cx)))
            .into_any_element()
    };
    let controls = match state {
        Some(State::Current) => vec![remove(cx)],
        Some(State::Outdated) => vec![install(cx, text(SETTINGS_AGENTS_UPDATE), true), remove(cx)],
        Some(State::Foreign) => vec![
            button("install", text(SETTINGS_AGENTS_INSTALL), true)
                .disabled(true)
                .tooltip(text(SETTINGS_AGENTS_SKILL_FOREIGN_TOOLTIP))
                .into_any_element(),
        ],
        // Covered: the agent reads a folder that has the skill; nothing to press.
        Some(State::NotInstalled) if row.covered.is_some() => Vec::new(),
        _ => vec![install(cx, text(SETTINGS_AGENTS_INSTALL), true)],
    };
    let problem = row.state.as_ref().err().map(|err| {
        div().pb_2().pl(px(style::ROW_TILE + 12.0)).child(style::notice(
            ui,
            style::NoticeKind::Danger,
            err.clone(),
        ))
    });
    let mut parts = style::TileRowParts::new(style::RowTile::Agent(target.agent), agent.name);
    parts.chip = chip.map(|(kind, word)| (kind, word.into()));
    parts.hint = Some(hint.into());
    parts.more = Some(
        format!(
            "{detail} {}. {}",
            row.covered.as_deref().unwrap_or(&row.shown),
            text(SETTINGS_AGENTS_SKILLS_DESCRIPTION)
        )
        .into(),
    );
    parts.controls = controls;
    div()
        .flex()
        .flex_col()
        .child(style::tile_row(SharedString::from(format!("agent-skill-{word}")), ui, parts))
        .children(problem)
        .into_any_element()
}

/// The button that copies the `claude mcp add …` line; the line itself is under the row, so it never
/// competes with the label for the row's width (see [`mcp_command`]).
fn draw_mcp(_: &SettingsWindow, _: &mut Context<SettingsWindow>) -> AnyElement {
    let copy = mcp_add_command();
    style::form_button("agent-mcp-copy")
        .outline()
        .icon(IconName::Copy)
        .label(text(SETTINGS_AGENTS_COPY_COMMAND))
        .on_click(move |_, _, cx| crate::platform::clipboard::write(copy.clone(), cx))
        .into_any_element()
}

/// The command, in a box across the column under the MCP row. It wraps rather than cut: it is what a person
/// pastes into a terminal.
fn mcp_command(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let ui = ui(cx);
    let (code_font, _) = crate::fonts::code_font(cx);
    Some(
        div()
            .px_2()
            .py_1()
            .rounded(ui.radius)
            .bg(ui.hover)
            .font_family(code_font)
            .text_caption()
            .text_color(ui.text)
            .child(mcp_add_command())
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::AgentId;
    use wormhole_core::agents::skill::{self, State};

    use super::load;

    /// A scratch home directory, removed at the end of the test.
    struct Home(std::path::PathBuf);

    impl Home {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("wormhole-agent-skills-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch home");
            Self(dir)
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_rows_read_the_skill_of_each_agent_from_its_folder() {
        let home = Home::new();
        let rows = load(&home.0);
        assert_eq!(rows.len(), skill::TARGETS.len());
        assert!(rows.iter().all(|row| row.state == Ok(State::NotInstalled)));
        let claude = skill::target(AgentId::Claude).expect("Claude Code loads skills");
        skill::install(claude, &home.0).expect("installs into the scratch home");
        let rows = load(&home.0);
        let row = rows.iter().find(|row| row.target.agent == AgentId::Claude).expect("its row");
        assert_eq!(
            (row.state.clone(), row.shown.as_str()),
            (Ok(State::Current), "~/.claude/skills/wormhole")
        );
        // Another agent's folder was not touched.
        assert!(
            rows.iter()
                .filter(|row| row.target.agent != AgentId::Claude)
                .all(|row| row.state == Ok(State::NotInstalled))
        );
    }
}
