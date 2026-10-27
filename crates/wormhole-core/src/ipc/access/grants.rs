//! The approvals that last a while and not for good (`agent-permissions.md`, 7.2 and 7.4): "for this
//! session" (the agent's process, until it exits), "until Wormhole quits", and the cool-down after a
//! "Don't Allow". They live in memory on purpose; "Always" is the store's.

use std::time::{Duration, Instant, SystemTime};

use super::caps::CapSet;
use super::identity::Identity;

/// How long a program that was told no is not asked again.
pub const COOL_DOWN: Duration = Duration::from_secs(120);

/// A running process, told from another that got the same pid by when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessRef {
    pub pid: u32,
    pub started: SystemTime,
}

/// How long an approval lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// While this process runs.
    Session(ProcessRef),
    /// Until Wormhole quits.
    UntilQuit,
}

/// One approval in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub id: u64,
    pub identity: Identity,
    /// The name it was approved under ("Claude Code").
    pub label: String,
    pub scope: Scope,
    pub caps: CapSet,
}

#[derive(Debug, Default)]
pub struct Grants {
    held: Vec<Held>,
    denied: Vec<(Identity, Instant)>,
    last_id: u64,
}

impl Grants {
    /// Approve `identity` for `scope`. The same identity approved again for the same kind of scope
    /// (and the same process) keeps its place and takes the new powers. The approval's id: what a
    /// stream started under it is revoked by.
    pub fn grant(&mut self, identity: Identity, label: String, scope: Scope, caps: CapSet) -> u64 {
        self.denied.retain(|(denied, _)| *denied != identity);
        if let Some(held) = self.held.iter_mut().find(|held| held.identity == identity && held.scope == scope)
        {
            held.caps = caps;
            held.label = label;
            return held.id;
        }
        self.last_id += 1;
        self.held.push(Held { id: self.last_id, identity, label, scope, caps });
        self.last_id
    }

    /// The approval that covers `identity` running as `process`, if any. One for the whole run of
    /// Wormhole wins over one for a session.
    pub fn find(&self, identity: &Identity, process: Option<ProcessRef>) -> Option<&Held> {
        let mut covering = self.held.iter().filter(|held| {
            held.identity == *identity
                && match held.scope {
                    Scope::UntilQuit => true,
                    Scope::Session(of) => process == Some(of),
                }
        });
        let first = covering.next()?;
        Some(covering.fold(first, |best, next| {
            if matches!(next.scope, Scope::UntilQuit) && !matches!(best.scope, Scope::UntilQuit) {
                next
            } else {
                best
            }
        }))
    }

    /// Note that `identity` was told no at `now`.
    pub fn deny(&mut self, identity: Identity, now: Instant) {
        self.denied.retain(|(denied, _)| *denied != identity);
        self.denied.push((identity, now));
    }

    /// Whether `identity` was told no so recently that it is not asked again.
    pub fn cooling(&self, identity: &Identity, now: Instant) -> bool {
        self.denied
            .iter()
            .any(|(denied, at)| denied == identity && now.saturating_duration_since(*at) < COOL_DOWN)
    }

    /// Change what approval `id` allows (the switches in Settings). Whether there was one.
    pub fn set_caps(&mut self, id: u64, caps: CapSet) -> bool {
        match self.held.iter_mut().find(|held| held.id == id) {
            Some(held) => {
                held.caps = caps;
                true
            }
            None => false,
        }
    }

    /// Take approval `id` back. Whether there was one.
    pub fn revoke(&mut self, id: u64) -> bool {
        let before = self.held.len();
        self.held.retain(|held| held.id != id);
        self.held.len() != before
    }

    /// Forget the session approvals whose process `running` says is gone, and cool-downs that are over.
    /// Returns the ids dropped.
    pub fn prune(&mut self, now: Instant, running: impl Fn(ProcessRef) -> bool) -> Vec<u64> {
        let mut dropped = Vec::new();
        self.held.retain(|held| match held.scope {
            Scope::Session(process) if !running(process) => {
                dropped.push(held.id);
                false
            }
            _ => true,
        });
        self.denied.retain(|(_, at)| now.saturating_duration_since(*at) < COOL_DOWN);
        dropped
    }

    /// Every approval in force, oldest first.
    pub fn held(&self) -> &[Held] {
        &self.held
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime};

    use super::{COOL_DOWN, Grants, ProcessRef, Scope};
    use crate::agents::AgentId;
    use crate::ipc::access::caps::CapSet;
    use crate::ipc::access::identity::{Identity, Kind};

    fn agent(name: AgentId) -> Identity {
        Identity { kind: Kind::AdHoc, agent: Some(name), pin: "h".into(), ..Identity::scripts() }
    }

    fn process(pid: u32, at: u64) -> ProcessRef {
        ProcessRef { pid, started: SystemTime::UNIX_EPOCH + Duration::from_secs(at) }
    }

    #[test]
    fn a_session_approval_covers_that_process_and_not_another_that_got_its_pid() {
        let mut grants = Grants::default();
        grants.grant(
            agent(AgentId::Claude),
            "Claude Code".into(),
            Scope::Session(process(10, 100)),
            CapSet::STANDARD,
        );
        assert!(grants.find(&agent(AgentId::Claude), Some(process(10, 100))).is_some());
        assert!(grants.find(&agent(AgentId::Claude), Some(process(10, 200))).is_none(), "the pid came back");
        assert!(grants.find(&agent(AgentId::Claude), Some(process(11, 100))).is_none());
        assert!(grants.find(&agent(AgentId::Claude), None).is_none());
        assert!(grants.find(&agent(AgentId::Codex), Some(process(10, 100))).is_none(), "another program");
    }

    #[test]
    fn an_approval_until_quit_covers_any_run_and_wins_over_a_session_one() {
        let mut grants = Grants::default();
        let session =
            grants.grant(agent(AgentId::Claude), "C".into(), Scope::Session(process(1, 1)), CapSet::EMPTY);
        let quit = grants.grant(agent(AgentId::Claude), "C".into(), Scope::UntilQuit, CapSet::STANDARD);
        assert_ne!(session, quit);
        let found = grants.find(&agent(AgentId::Claude), Some(process(1, 1))).expect("covered");
        assert_eq!(found.id, quit);
        assert!(grants.find(&agent(AgentId::Claude), Some(process(99, 9))).is_some());
        // Approving again renews rather than adds.
        assert_eq!(grants.grant(agent(AgentId::Claude), "C".into(), Scope::UntilQuit, CapSet::EMPTY), quit);
        assert_eq!(grants.held().len(), 2);
    }

    #[test]
    fn a_no_cools_down_for_two_minutes_and_a_yes_ends_it() {
        let mut grants = Grants::default();
        let now = Instant::now();
        let who = agent(AgentId::Aider);
        assert!(!grants.cooling(&who, now));
        grants.deny(who.clone(), now);
        assert!(grants.cooling(&who, now + Duration::from_secs(119)));
        assert!(!grants.cooling(&who, now + COOL_DOWN), "two minutes are over");
        assert!(!grants.cooling(&agent(AgentId::Claude), now), "only that program");
        grants.grant(who.clone(), "Aider".into(), Scope::UntilQuit, CapSet::STANDARD);
        assert!(!grants.cooling(&who, now), "the user changed their mind in Settings");
    }

    #[test]
    fn what_an_approval_allows_can_be_changed_and_a_missing_one_cannot() {
        let mut grants = Grants::default();
        let id = grants.grant(agent(AgentId::Claude), "C".into(), Scope::UntilQuit, CapSet::STANDARD);
        assert!(grants.set_caps(id, CapSet::EMPTY));
        assert_eq!(grants.held()[0].caps, CapSet::EMPTY);
        assert!(!grants.set_caps(id + 1, CapSet::STANDARD));
    }

    #[test]
    fn pruning_drops_the_sessions_of_processes_that_ended_and_reports_them() {
        let mut grants = Grants::default();
        let gone =
            grants.grant(agent(AgentId::Claude), "C".into(), Scope::Session(process(1, 1)), CapSet::STANDARD);
        let alive =
            grants.grant(agent(AgentId::Codex), "X".into(), Scope::Session(process(2, 2)), CapSet::STANDARD);
        let quit = grants.grant(agent(AgentId::Pi), "P".into(), Scope::UntilQuit, CapSet::STANDARD);
        let dropped = grants.prune(Instant::now(), |p| p.pid == 2);
        assert_eq!(dropped, [gone]);
        let ids: Vec<u64> = grants.held().iter().map(|h| h.id).collect();
        assert_eq!(ids, [alive, quit]);
        assert!(grants.revoke(alive) && !grants.revoke(alive));
    }
}
