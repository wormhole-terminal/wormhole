//! What the host remembers of its program for a client that attaches later: the title, the
//! folder the shell reported and the OSC 7501 reports that still stand.
//!
//! A local terminal's UI hears each of these once, as it happens, and keeps them. A client that
//! attaches to a host that already ran for a while (the app relaunched) was not there, and the
//! screen it is repainted with cannot say them (a replica drops its own events). So the host
//! replays them as ordinary events on every attach, and a replayed one lands exactly where a
//! live one would.
//!
//! The status is not one body: `Shared`'s store holds a record per id, so the memory keeps the
//! body of each record that stands, oldest first, and ends them as the store does (a clear, a
//! full reset, a new prompt, the program's exit). Replayed in order, the bodies leave a client's
//! store as the host's would be.

use super::event::HostEvent;
use crate::program_status::{self, Parsed, RECORDS_MAX, Report, State};

/// One OSC 7501 record that stands: where it goes, how long it lasts, and the body that made it.
struct Standing {
    id: Option<String>,
    state: State,
    body: String,
}

#[derive(Default)]
pub(super) struct Memory {
    title: Option<String>,
    cwd: Option<String>,
    status: Vec<Standing>,
}

impl Memory {
    pub(super) fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_owned());
    }

    pub(super) fn clear_title(&mut self) {
        self.title = None;
    }

    pub(super) fn set_cwd(&mut self, path: &str) {
        self.cwd = Some(path.to_owned());
    }

    /// An accepted report's body; read as a client reads it, so what is kept is what it keeps.
    pub(super) fn report(&mut self, body: &str) {
        let Some(Parsed::Report(report)) = program_status::parse(body.as_bytes(), 1) else { return };
        match report {
            Report::Clear { id: None } => self.status.clear(),
            Report::Clear { id: Some(id) } => {
                self.status.retain(|s| !s.id.as_deref().is_some_and(|stored| within(stored, &id)))
            }
            Report::Set { id, record } => {
                match self.status.iter().position(|s| s.id == id) {
                    Some(at) => {
                        self.status.remove(at);
                    }
                    None if self.status.len() >= RECORDS_MAX => {
                        self.status.remove(0);
                    }
                    None => {}
                }
                self.status.push(Standing { id, state: record.state, body: body.to_owned() });
            }
        }
    }

    /// A new prompt, or the program's exit: only what outlives the program stands.
    pub(super) fn program_over(&mut self) {
        self.status.retain(|s| s.state.outlives_program());
    }

    /// RIS: the title and every record are gone (the folder is not: a reset does not move the
    /// shell).
    pub(super) fn full_reset(&mut self) {
        self.title = None;
        self.status.clear();
    }

    /// The events that bring a client to this state, in the order to deliver them.
    pub(super) fn events(&self) -> Vec<HostEvent> {
        let title = self.title.iter().map(|t| HostEvent::Title(t.clone()));
        let cwd = self.cwd.iter().map(|p| HostEvent::Cwd(p.clone()));
        let status = self.status.iter().map(|s| HostEvent::ProgramStatus(s.body.clone()));
        title.chain(cwd).chain(status).collect()
    }
}

/// `id` is `ancestor` or beneath it (`a/b` is within `a`).
fn within(id: &str, ancestor: &str) -> bool {
    id.strip_prefix(ancestor).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bodies(memory: &Memory) -> Vec<String> {
        memory
            .events()
            .into_iter()
            .filter_map(|e| match e {
                HostEvent::ProgramStatus(body) => Some(body),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn events_come_title_then_folder_then_records() {
        let mut memory = Memory::default();
        memory.report("state=working");
        memory.set_cwd("/tmp");
        memory.set_title("hello");
        assert_eq!(
            memory.events(),
            vec![
                HostEvent::Title("hello".into()),
                HostEvent::Cwd("/tmp".into()),
                HostEvent::ProgramStatus("state=working".into())
            ]
        );
    }

    #[test]
    fn a_record_replaces_its_own_and_moves_last() {
        let mut memory = Memory::default();
        memory.report("state=working:id=a");
        memory.report("state=working:id=b");
        memory.report("state=blocked:kind=question:id=a");
        assert_eq!(bodies(&memory), ["state=working:id=b", "state=blocked:kind=question:id=a"]);
    }

    #[test]
    fn a_clear_ends_the_record_and_its_children() {
        let mut memory = Memory::default();
        memory.report("state=working:id=a");
        memory.report("state=working:id=a/b");
        memory.report("state=working:id=ab");
        memory.report("state=clear:id=a");
        assert_eq!(bodies(&memory), ["state=working:id=ab"]);
        memory.report("state=clear");
        assert!(bodies(&memory).is_empty());
    }

    #[test]
    fn a_prompt_or_the_exit_keeps_only_what_outlives_the_program() {
        let mut memory = Memory::default();
        memory.report("state=working:id=a");
        memory.report("state=done:id=b");
        memory.report("state=idle:id=c");
        memory.program_over();
        assert_eq!(bodies(&memory), ["state=done:id=b"]);
    }

    #[test]
    fn a_full_reset_forgets_the_title_and_records_but_not_the_folder() {
        let mut memory = Memory::default();
        memory.set_title("t");
        memory.set_cwd("/tmp");
        memory.report("state=done");
        memory.full_reset();
        assert_eq!(memory.events(), vec![HostEvent::Cwd("/tmp".into())]);
    }

    #[test]
    fn what_a_client_would_discard_is_not_kept() {
        let mut memory = Memory::default();
        memory.report("state=sleepy");
        memory.report("?");
        assert!(memory.events().is_empty());
    }
}
