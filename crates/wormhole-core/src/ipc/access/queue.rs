//! The questions waiting for the user (`agent-permissions.md`, 7.4): one on screen at a time, in the
//! order they came; a program that asks again while its question is open waits behind the same one and is
//! answered by it.

use std::collections::VecDeque;
use std::sync::mpsc::SyncSender;

use super::caps::Need;
use super::identity::Identity;
use super::resolve::Resolved;
use crate::ipc::ask_job::Decision;

/// How many different questions may wait. More are refused (`too_many_questions`).
pub const QUEUE_MAX: usize = crate::ipc::control::ASK_QUEUE_MAX;

/// A question, and the requests waiting for its answer.
#[derive(Debug)]
pub struct Pending {
    pub id: u64,
    pub resolved: Resolved,
    /// Whose it is when there are several places that decide separately (the app's profiles): questions
    /// of different scopes are never one question.
    pub scope: u64,
    /// What the first request that raised it needed.
    pub need: Need,
    waiters: Vec<SyncSender<Decision>>,
    shown: bool,
}

impl Pending {
    pub fn identity(&self) -> &Identity {
        &self.resolved.identity
    }

    /// Answer every request that waited, each with its own decision.
    pub fn answer(self, mut decision: impl FnMut() -> Decision) {
        for waiter in self.waiters {
            let _ = waiter.try_send(decision());
        }
    }
}

/// Too many questions are waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

#[derive(Debug, Default)]
pub struct Queue {
    pending: VecDeque<Pending>,
    last_id: u64,
}

impl Queue {
    /// Queue a question for `resolved` in `scope`, or join the one its program has open there. `waiter` is the
    /// request that waits for the answer, if it does (told at once if the question is on the screen already).
    /// The question's id.
    pub fn push(
        &mut self,
        resolved: Resolved,
        need: Need,
        scope: u64,
        waiter: Option<SyncSender<Decision>>,
    ) -> Result<u64, Full> {
        if let Some(open) =
            self.pending.iter_mut().find(|p| p.scope == scope && p.resolved.identity == resolved.identity)
        {
            if let (true, Some(waiter)) = (open.shown, &waiter) {
                let _ = waiter.try_send(Decision::Shown);
            }
            open.waiters.extend(waiter);
            return Ok(open.id);
        }
        if self.pending.len() >= QUEUE_MAX {
            return Err(Full);
        }
        self.last_id += 1;
        self.pending.push_back(Pending {
            id: self.last_id,
            resolved,
            scope,
            need,
            waiters: waiter.into_iter().collect(),
            shown: false,
        });
        Ok(self.last_id)
    }

    /// The question to put on screen now: the first that is not shown, and only when none is. Marks it shown.
    pub fn next_to_show(&mut self) -> Option<&Pending> {
        if self.pending.iter().any(|p| p.shown) {
            return None;
        }
        let next = self.pending.iter_mut().find(|p| !p.shown)?;
        next.shown = true;
        // The minute of every request that waits for it starts now.
        for waiter in &next.waiters {
            let _ = waiter.try_send(Decision::Shown);
        }
        Some(next)
    }

    /// Take question `id` off the queue to answer it.
    pub fn take(&mut self, id: u64) -> Option<Pending> {
        let at = self.pending.iter().position(|p| p.id == id)?;
        self.pending.remove(at)
    }

    /// Every question of `scope`, to settle them all (what decides there changed).
    pub fn take_scope(&mut self, scope: u64) -> Vec<Pending> {
        let (taken, kept): (VecDeque<_>, VecDeque<_>) =
            std::mem::take(&mut self.pending).into_iter().partition(|p| p.scope == scope);
        self.pending = kept;
        taken.into_iter().collect()
    }

    /// Every question, to refuse them all (Wormhole is quitting, or access was switched off).
    pub fn drain(&mut self) -> Vec<Pending> {
        self.pending.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pending> {
        self.pending.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::{Full, QUEUE_MAX, Queue};
    use crate::agents::AgentId;
    use crate::ipc::access::caps::{Capability, Need};
    use crate::ipc::access::identity::{Identity, Kind};
    use crate::ipc::access::resolve::Resolved;
    use crate::ipc::ask_job::Decision;

    fn resolved(pin: &str) -> Resolved {
        Resolved {
            identity: Identity {
                kind: Kind::AdHoc,
                agent: Some(AgentId::Claude),
                pin: pin.into(),
                ..Identity::scripts()
            },
            label: "Claude Code".into(),
            process: None,
            program: None,
            elevated: false,
            started_by: None,
            unusual_signer: false,
        }
    }

    const TYPE: Need = Need::Capability(Capability::Type);

    #[test]
    fn one_question_is_on_screen_at_a_time_in_the_order_they_came() {
        let mut queue = Queue::default();
        let a = queue.push(resolved("a"), TYPE, 0, None).expect("room");
        let b = queue.push(resolved("b"), TYPE, 0, None).expect("room");
        assert_eq!(queue.next_to_show().map(|p| p.id), Some(a));
        assert!(queue.next_to_show().is_none(), "a is still up");
        queue.take(a).expect("a");
        assert_eq!(queue.next_to_show().map(|p| p.id), Some(b));
    }

    #[test]
    fn a_program_that_asks_again_joins_its_open_question_and_all_its_waiters_are_answered() {
        let mut queue = Queue::default();
        let (first, first_rx) = mpsc::sync_channel(1);
        let (second, second_rx) = mpsc::sync_channel(1);
        let id = queue.push(resolved("a"), TYPE, 0, Some(first)).expect("room");
        assert_eq!(queue.push(resolved("a"), TYPE, 0, Some(second)).expect("joins"), id);
        assert_eq!(queue.len(), 1);
        queue.take(id).expect("there").answer(|| Decision::Retry);
        assert!(matches!(first_rx.try_recv(), Ok(Decision::Retry)));
        assert!(matches!(second_rx.try_recv(), Ok(Decision::Retry)));
    }

    #[test]
    fn a_waiter_is_told_when_its_question_is_shown_and_a_late_joiner_at_once() {
        let mut queue = Queue::default();
        let (first, first_rx) = mpsc::sync_channel(4);
        let id = queue.push(resolved("a"), TYPE, 0, Some(first)).expect("room");
        assert!(first_rx.try_recv().is_err(), "queued, not shown: its minute has not started");
        assert_eq!(queue.next_to_show().map(|p| p.id), Some(id));
        assert!(matches!(first_rx.try_recv(), Ok(Decision::Shown)));
        let (late, late_rx) = mpsc::sync_channel(4);
        queue.push(resolved("a"), TYPE, 0, Some(late)).expect("joins");
        assert!(matches!(late_rx.try_recv(), Ok(Decision::Shown)), "it joined one that is up");
    }

    #[test]
    fn the_same_program_in_another_scope_is_another_question_and_a_scope_can_be_taken_whole() {
        let mut queue = Queue::default();
        let one = queue.push(resolved("a"), TYPE, 1, None).expect("room");
        let two = queue.push(resolved("a"), TYPE, 2, None).expect("room");
        assert_ne!(one, two);
        let taken = queue.take_scope(2);
        assert_eq!(taken.iter().map(|p| p.id).collect::<Vec<_>>(), [two]);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn more_than_the_limit_of_different_programs_are_refused() {
        let mut queue = Queue::default();
        for n in 0..QUEUE_MAX {
            queue.push(resolved(&n.to_string()), TYPE, 0, None).expect("room");
        }
        assert_eq!(queue.push(resolved("one more"), TYPE, 0, None), Err(Full));
        // A program that is queued already can still join.
        assert!(queue.push(resolved("0"), TYPE, 0, None).is_ok());
        assert_eq!(queue.drain().len(), QUEUE_MAX);
        assert!(queue.is_empty());
    }
}
