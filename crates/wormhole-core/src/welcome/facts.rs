//! The facts the pages show, made from the machine: the agents found (their hooks read, never
//! written). The app answers the program's `State` with these, and a read-only tour makes its own, so
//! a page reads the same either way.

use std::path::{Path, PathBuf};

use crate::agents::integrations::{self, State};
use crate::agents::{self, AGENTS};
use crate::intro::Handover;
use crate::ipc::control::{AgentFact, AgentsFacts, HooksFact, LookFacts, WelcomeState};

/// The agents on PATH, each with the state of its hooks under `home`, and how many others are
/// recognised. A few hundred `stat`s and a few small reads, so off the UI thread.
pub fn scan_agents(home: &Path) -> AgentsFacts {
    let mut facts = AgentsFacts::default();
    for agent in AGENTS {
        if !agents::installed(agent) {
            facts.others += 1;
            continue;
        }
        let spec = integrations::spec(agent.id);
        facts.found.push(AgentFact {
            id: agent.id,
            name: agent.name.to_string(),
            hooks: match spec.map(|spec| spec.state(home)) {
                None => HooksFact::Unsupported,
                Some(Ok(State::Installed)) => HooksFact::Installed,
                Some(Ok(State::Partial)) => HooksFact::Partial,
                Some(Ok(State::NotInstalled) | Err(_)) => HooksFact::Missing,
            },
            path: spec.map(|spec| spec.display_path(home)),
        });
    }
    facts
}

/// The state a read-only tour shows: nothing from an app, so the agents the machine has, and no look
/// or notification choices to change.
pub fn tour_state(home: &Path) -> WelcomeState {
    WelcomeState {
        wait_ms: 0,
        reduce_motion: false,
        handover: Handover::default(),
        first_run: true,
        look: LookFacts {
            themes: Vec::new(),
            theme: String::new(),
            primaries: Vec::new(),
            primary: String::new(),
            font: String::new(),
            font_size: 0.0,
            ligatures: false,
        },
        notify: None,
        agents: Some(scan_agents(home)),
        problem: None,
    }
}

/// The home folder, as the program sees it.
pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch home, removed at the end of the test.
    struct Home(PathBuf);

    impl Home {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("wormhole-welcome-{name}-{}", std::process::id()));
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
    fn looking_for_agents_lists_each_once_and_writes_nothing() {
        let home = Home::new("scan");
        let facts = scan_agents(&home.0);
        assert_eq!(facts.found.len() + facts.others, AGENTS.len());
        assert_eq!(std::fs::read_dir(&home.0).expect("readable").count(), 0, "the scan wrote into the home");
        for found in &facts.found {
            // Nothing installed in an empty home: every agent with hooks offers an Install.
            assert!(matches!(found.hooks, HooksFact::Missing | HooksFact::Unsupported), "{found:?}");
        }
    }

    #[test]
    fn a_tour_offers_the_agents_and_no_choices() {
        let home = Home::new("tour");
        let state = tour_state(&home.0);
        assert!(state.first_run && state.wait_ms == 0 && !state.reduce_motion);
        assert!(state.look.themes.is_empty() && state.notify.is_none());
        assert!(state.agents.is_some());
    }
}
