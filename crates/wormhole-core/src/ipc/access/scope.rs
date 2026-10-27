//! Which terminals a grant reaches (`agent-permissions.md`, section 6). Within a grant, `read`, `type`
//! and `run` apply by default only to the terminals the program opened itself and to the one it runs in;
//! any other terminal is out of scope until the user says so for that terminal, for a while. Pure: the
//! app tells it what is opened and closed, and which terminals a request touches.
//!
//! Nothing here is stored. Ownership and the user's answers about single terminals live in memory, so
//! after a restart the terminals a program opened earlier are "others", which is the safe reading.

use std::collections::HashMap;
use std::time::Instant;

use super::caps::{CapSet, Capability};
use super::grants::{COOL_DOWN, ProcessRef, Scope};
use super::identity::Identity;

/// What the user allowed for one terminal, for how long.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Widened {
    identity: Identity,
    scope: Scope,
    terminal: i64,
    caps: CapSet,
}

/// The terminals each program owns, the single terminals the user opened up to it, and the single
/// terminals it was told no about a moment ago.
#[derive(Debug, Default)]
pub struct Scopes {
    owned: HashMap<i64, Identity>,
    widened: Vec<Widened>,
    denied: Vec<(Identity, i64, Instant)>,
}

impl Scopes {
    /// `terminal` was opened by `identity`'s request: it is its own, with every capability it holds.
    pub fn own(&mut self, terminal: i64, identity: Identity) {
        self.owned.insert(terminal, identity);
    }

    /// The program that opened `terminal`, while it is open.
    pub fn owner(&self, terminal: i64) -> Option<&Identity> {
        self.owned.get(&terminal)
    }

    /// The terminals `identity` opened that are still open.
    pub fn owned_by(&self, identity: &Identity) -> Vec<i64> {
        let mut terminals: Vec<i64> = self
            .owned
            .iter()
            .filter(|(_, owner)| *owner == identity)
            .map(|(terminal, _)| *terminal)
            .collect();
        terminals.sort_unstable();
        terminals
    }

    /// The user let `identity` use `caps` on `terminal` for `scope`. What it was told no about for that
    /// terminal is forgotten. Asking again for the same grant renews it with the new powers.
    pub fn widen(&mut self, identity: Identity, scope: Scope, terminal: i64, caps: CapSet) {
        self.denied.retain(|(denied, id, _)| !(*denied == identity && *id == terminal));
        if let Some(held) = self
            .widened
            .iter_mut()
            .find(|held| held.identity == identity && held.scope == scope && held.terminal == terminal)
        {
            held.caps = CapSet::from_bits(held.caps.bits() | caps.bits());
            return;
        }
        self.widened.push(Widened { identity, scope, terminal, caps });
    }

    /// Whether `identity`, running as `process`, may use `capability` on `terminal` by what it opened or
    /// by what the user opened up for it. A `run` allowed on a terminal reads and types there too: it
    /// types the command and reads what it prints.
    pub fn covers(
        &self,
        identity: &Identity,
        process: Option<ProcessRef>,
        terminal: i64,
        capability: Capability,
    ) -> bool {
        if self.owned.get(&terminal) == Some(identity) {
            return true;
        }
        self.widened.iter().any(|held| {
            held.terminal == terminal
                && held.identity == *identity
                && match held.scope {
                    Scope::UntilQuit => true,
                    Scope::Session(of) => process == Some(of),
                }
                && (held.caps.contains(capability)
                    || (held.caps.contains(Capability::Run)
                        && matches!(capability, Capability::Read | Capability::Type)))
        })
    }

    /// What of `touched` is out of scope: not the caller's own terminal (`own`, found by its place in the
    /// process tree), not owned and not opened up for what the request needs.
    pub fn out_of_scope(
        &self,
        identity: &Identity,
        process: Option<ProcessRef>,
        own: &[i64],
        touched: &[(i64, Capability)],
    ) -> Vec<(i64, Capability)> {
        touched
            .iter()
            .filter(|(terminal, capability)| {
                !own.contains(terminal) && !self.covers(identity, process, *terminal, *capability)
            })
            .copied()
            .collect()
    }

    /// Which of `terminals` the program may read the text of: the ones it is in, owns, or was allowed.
    pub fn readable(
        &self,
        identity: &Identity,
        process: Option<ProcessRef>,
        own: &[i64],
        terminals: impl IntoIterator<Item = i64>,
    ) -> Vec<i64> {
        terminals
            .into_iter()
            .filter(|terminal| {
                own.contains(terminal) || self.covers(identity, process, *terminal, Capability::Read)
            })
            .collect()
    }

    /// Only the terminals in `open` are open: whatever was said about any other is dropped (they closed).
    pub fn retain_open(&mut self, open: &[i64]) {
        self.owned.retain(|terminal, _| open.contains(terminal));
        self.widened.retain(|held| open.contains(&held.terminal));
        self.denied.retain(|(_, terminal, _)| open.contains(terminal));
    }

    /// `terminal` was closed: nothing about it is kept.
    pub fn forget(&mut self, terminal: i64) {
        self.owned.remove(&terminal);
        self.widened.retain(|held| held.terminal != terminal);
        self.denied.retain(|(_, id, _)| *id != terminal);
    }

    /// Note that `identity` was told no about `terminal` at `now`.
    pub fn deny(&mut self, identity: Identity, terminal: i64, now: Instant) {
        self.denied.retain(|(denied, id, _)| !(*denied == identity && *id == terminal));
        self.denied.push((identity, terminal, now));
    }

    /// Whether `identity` was told no about `terminal` so recently that it is not asked again.
    pub fn cooling(&self, identity: &Identity, terminal: i64, now: Instant) -> bool {
        self.denied.iter().any(|(denied, id, at)| {
            denied == identity && *id == terminal && now.saturating_duration_since(*at) < COOL_DOWN
        })
    }

    /// Forget what a session allowed for a process that `running` says is gone, and cool-downs that are
    /// over. Whether anything allowed was dropped.
    pub fn prune(&mut self, now: Instant, running: impl Fn(ProcessRef) -> bool) -> bool {
        let before = self.widened.len();
        self.widened.retain(|held| match held.scope {
            Scope::Session(process) => running(process),
            Scope::UntilQuit => true,
        });
        self.denied.retain(|(_, _, at)| now.saturating_duration_since(*at) < COOL_DOWN);
        self.widened.len() != before
    }

    /// Every terminal a single answer opened up, with its program and what it allows (what Settings could
    /// show); oldest first.
    pub fn widened(&self) -> impl Iterator<Item = (&Identity, i64, CapSet)> {
        self.widened.iter().map(|held| (&held.identity, held.terminal, held.caps))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime};

    use super::{CapSet, Capability, ProcessRef, Scope, Scopes};
    use crate::agents::AgentId;
    use crate::ipc::access::identity::{Identity, Kind};

    use Capability::{Read, Run, Type};

    fn agent(name: AgentId) -> Identity {
        Identity { kind: Kind::AdHoc, agent: Some(name), pin: "h".into(), ..Identity::scripts() }
    }

    fn process(pid: u32) -> ProcessRef {
        ProcessRef { pid, started: SystemTime::UNIX_EPOCH + Duration::from_secs(1) }
    }

    /// The table of section 6.1: what is opened, what is somebody else's, what the agent runs in.
    #[test]
    fn a_program_reaches_what_it_opened_and_where_it_runs_and_nothing_else() {
        let (claude, codex) = (agent(AgentId::Claude), agent(AgentId::Codex));
        let mut scopes = Scopes::default();
        scopes.own(10, claude.clone());
        scopes.own(11, codex.clone());
        let touched = |ids: &[i64], cap| ids.iter().map(|id| (*id, cap)).collect::<Vec<_>>();
        // 10 is Claude's own opening, 11 is Codex's, 12 was opened by the user, 13 is where Claude runs.
        for cap in [Read, Type, Run, Capability::Open, Capability::Close] {
            let out = scopes.out_of_scope(&claude, None, &[13], &touched(&[10, 11, 12, 13], cap));
            assert_eq!(out.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [11, 12], "{cap:?}");
        }
        assert_eq!(scopes.owned_by(&claude), [10]);
        assert_eq!(scopes.owner(11), Some(&codex));
        // Scripts own what a script opened, as everyone else does.
        scopes.own(14, Identity::scripts());
        assert!(scopes.covers(&Identity::scripts(), None, 14, Type));
        assert!(!scopes.covers(&claude, None, 14, Type));
    }

    #[test]
    fn a_split_of_an_owned_terminal_and_one_that_is_not_is_in_scope_only_where_it_is_owned() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        // The agent opened 20 and split it (21 is its own too); 22 was in the split already.
        scopes.own(20, claude.clone());
        scopes.own(21, claude.clone());
        let out = scopes.out_of_scope(&claude, None, &[], &[(20, Read), (21, Read), (22, Read)]);
        assert_eq!(out, [(22, Read)]);
    }

    #[test]
    fn this_session_opens_up_that_terminal_for_those_capabilities_and_only_for_that_run() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        scopes.widen(claude.clone(), Scope::Session(process(5)), 30, CapSet::EMPTY.with(Read));
        assert!(scopes.covers(&claude, Some(process(5)), 30, Read));
        assert!(!scopes.covers(&claude, Some(process(5)), 30, Type), "only what was asked for");
        assert!(!scopes.covers(&claude, Some(process(5)), 31, Read), "only that terminal");
        assert!(!scopes.covers(&claude, Some(process(6)), 30, Read), "another run of the program");
        assert!(!scopes.covers(&agent(AgentId::Codex), Some(process(5)), 30, Read), "another program");
        // Until Wormhole quits covers any run.
        scopes.widen(claude.clone(), Scope::UntilQuit, 31, CapSet::EMPTY.with(Type));
        assert!(scopes.covers(&claude, Some(process(99)), 31, Type));
        assert!(scopes.covers(&claude, None, 31, Type));
        // Asking again for more renews the same grant.
        scopes.widen(claude.clone(), Scope::UntilQuit, 31, CapSet::EMPTY.with(Read));
        assert!(scopes.covers(&claude, None, 31, Type) && scopes.covers(&claude, None, 31, Read));
    }

    #[test]
    fn running_a_command_in_a_terminal_reads_and_types_there_too() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        scopes.widen(claude.clone(), Scope::UntilQuit, 40, CapSet::EMPTY.with(Run));
        assert!(scopes.covers(&claude, None, 40, Read) && scopes.covers(&claude, None, 40, Type));
        assert!(!scopes.covers(&claude, None, 40, Capability::Open));
    }

    #[test]
    fn a_closed_terminal_drops_out_with_everything_said_about_it() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        scopes.own(50, claude.clone());
        scopes.widen(claude.clone(), Scope::UntilQuit, 51, CapSet::STANDARD);
        scopes.deny(claude.clone(), 52, Instant::now());
        scopes.forget(50);
        scopes.forget(51);
        scopes.forget(52);
        assert!(scopes.owned_by(&claude).is_empty());
        assert_eq!(scopes.widened().count(), 0);
        assert!(!scopes.cooling(&claude, 52, Instant::now()));
    }

    #[test]
    fn terminals_that_are_no_longer_open_are_dropped_wherever_they_were_kept() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        scopes.own(1, claude.clone());
        scopes.own(2, claude.clone());
        scopes.widen(claude.clone(), Scope::UntilQuit, 3, CapSet::STANDARD);
        scopes.widen(claude.clone(), Scope::UntilQuit, 4, CapSet::STANDARD);
        scopes.deny(claude.clone(), 5, Instant::now());
        scopes.retain_open(&[2, 4]);
        assert_eq!(scopes.owned_by(&claude), [2]);
        assert_eq!(scopes.widened().map(|(_, terminal, _)| terminal).collect::<Vec<_>>(), [4]);
        assert!(!scopes.cooling(&claude, 5, Instant::now()));
    }

    #[test]
    fn a_no_cools_down_for_that_terminal_only_and_a_yes_ends_it() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        let now = Instant::now();
        scopes.deny(claude.clone(), 60, now);
        assert!(scopes.cooling(&claude, 60, now + Duration::from_secs(119)));
        assert!(!scopes.cooling(&claude, 60, now + Duration::from_secs(120)));
        assert!(!scopes.cooling(&claude, 61, now), "another terminal is asked about");
        assert!(!scopes.cooling(&agent(AgentId::Codex), 60, now), "another program is asked");
        scopes.widen(claude.clone(), Scope::UntilQuit, 60, CapSet::EMPTY.with(Read));
        assert!(!scopes.cooling(&claude, 60, now));
    }

    #[test]
    fn pruning_forgets_the_sessions_of_programs_that_ended_and_what_nobody_can_use() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        scopes.widen(claude.clone(), Scope::Session(process(1)), 70, CapSet::EMPTY.with(Read));
        scopes.widen(claude.clone(), Scope::Session(process(2)), 71, CapSet::EMPTY.with(Read));
        scopes.widen(claude.clone(), Scope::UntilQuit, 72, CapSet::EMPTY.with(Read));
        assert!(scopes.prune(Instant::now(), |p| p.pid == 2));
        let left: Vec<i64> = scopes.widened().map(|(_, terminal, _)| terminal).collect();
        assert_eq!(left, [71, 72]);
        assert!(!scopes.prune(Instant::now(), |_| true));
    }

    #[test]
    fn the_readable_terminals_of_a_stream_are_the_ones_in_scope() {
        let claude = agent(AgentId::Claude);
        let mut scopes = Scopes::default();
        scopes.own(80, claude.clone());
        scopes.widen(claude.clone(), Scope::UntilQuit, 81, CapSet::EMPTY.with(Read));
        scopes.widen(claude.clone(), Scope::UntilQuit, 82, CapSet::EMPTY.with(Type));
        assert_eq!(scopes.readable(&claude, None, &[83], [80, 81, 82, 83, 84]), [80, 81, 83]);
    }
}
