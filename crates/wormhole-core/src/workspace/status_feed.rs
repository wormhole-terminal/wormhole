//! The model's side of the status machines: they are fed from the shell's events, with what the window
//! knows (which terminals are on screen, whether it is in front) added to each step.

use super::event::Event;
use super::facts::Millis;
use super::model::WorkspaceModel;
use super::status::{Ask, Context, StatusEvent, StatusMachine};

impl WorkspaceModel {
    /// Something happened to terminal `id` at `at` (the shell's clock): the status machine decides
    /// what it means. A change that shows on its row is a `Listing`. Unknown ids are ignored (a
    /// terminal that just closed). What only the shell can do about it is in [`take_asks`](Self::take_asks).
    pub fn feed(&mut self, id: i64, event: StatusEvent, at: Millis) -> Vec<Event> {
        let Some(ix) = self.index_of(id) else { return Vec::new() };
        let cx = self.status_context(ix);
        let book = &mut self.status;
        book.clock = book.clock.max(at);
        if book.machines.len() > self.sessions.len() {
            let sessions = &self.sessions;
            book.machines.retain(|id, _| sessions.iter().any(|session| session.id == *id));
        }
        let machine = book.machines.entry(id).or_insert_with(|| StatusMachine::new(at));
        let facts = &mut self.sessions[ix].facts;
        let before = facts.clone();
        let looked_at = matches!(event, StatusEvent::Seen);
        let mut asks = vec![machine.step(facts, event, at, cx)];
        // What is in front of the person is seen as it happens (the app's tick does this every 0.4 s).
        if cx.looking() && !looked_at {
            asks.push(machine.step(facts, StatusEvent::Seen, at, cx));
        }
        book.asks.extend(asks.into_iter().flatten().map(|ask| (id, ask)));
        if *facts != before {
            self.emit(Event::Listing);
        }
        self.out.take()
    }

    /// When the soonest terminal wants a `StatusEvent::Tick`, and for which: the shell arms one timer
    /// for it and feeds the tick when it fires.
    pub fn next_due(&self) -> Option<(i64, Millis)> {
        self.sessions
            .iter()
            .filter_map(|session| {
                let due = self.status.machines.get(&session.id)?.next_due(&session.facts)?;
                Some((session.id, due))
            })
            .min_by_key(|&(_, due)| due)
    }

    /// What the machines need the shell to do, once: read the programs' reports again after the
    /// shell dropped what the program that left, or what the person saw, took with it, and feed the
    /// answer as `StatusEvent::Reported`.
    pub fn take_asks(&mut self) -> Vec<(i64, Ask)> {
        std::mem::take(&mut self.status.asks)
    }

    /// The window came to the front or went behind. In front, what is on screen is looked at.
    pub fn set_window_active(&mut self, active: bool, at: Millis) -> Vec<Event> {
        self.status.clock = self.status.clock.max(at);
        if std::mem::replace(&mut self.status.window_active, active) != active
            && active
            && self.acknowledge_on_screen()
        {
            self.emit(Event::Listing);
        }
        self.out.take()
    }

    fn status_context(&self, ix: usize) -> Context {
        Context { on_screen: self.shown().contains(&ix), window_active: self.status.window_active }
    }

    /// The terminals in front of the person: the selected one and the panes shown with it (all of a
    /// split, or only its zoomed pane).
    pub(in crate::workspace) fn shown(&self) -> Vec<usize> {
        let Some(selected) = self.selected_index() else { return Vec::new() };
        match self.sessions[selected].split.and_then(|id| self.split_of(id)) {
            Some(split) => {
                let zoomed = split.layout.zoomed();
                let id = split.id;
                self.members(id)
                    .into_iter()
                    .filter(|&ix| zoomed.is_none_or(|zoomed| zoomed == self.sessions[ix].id))
                    .collect()
            }
            None => vec![selected],
        }
    }

    /// The ids of [`shown`](Self::shown): which terminals a session host is to feed, and which stay headless.
    pub fn shown_ids(&self) -> Vec<i64> {
        self.shown().into_iter().map(|ix| self.sessions[ix].id).collect()
    }

    /// What is in front of the person counts as seen: each machine spends a finished or failed
    /// result and hears the bell. A question an agent asks stays until it is answered. Whether
    /// anything changed.
    pub(in crate::workspace) fn acknowledge_on_screen(&mut self) -> bool {
        let (at, mut changed) = (self.status.clock, false);
        for ix in self.shown() {
            let id = self.sessions[ix].id;
            let before = self.sessions[ix].facts.clone();
            let cx = self.status_context(ix);
            let machine = self.status.machines.entry(id).or_insert_with(|| StatusMachine::new(at));
            if let Some(ask) = machine.step(&mut self.sessions[ix].facts, StatusEvent::Seen, at, cx) {
                self.status.asks.push((id, ask));
            }
            changed |= self.sessions[ix].facts != before;
        }
        changed
    }
}
