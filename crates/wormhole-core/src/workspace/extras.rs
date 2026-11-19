//! The title row's two looked-up facts: the ports a terminal's programs listen on, and its branch's pull
//! request. The model only holds them and says which are wanted; a shell looks them up (ports from the
//! shell's process tree, the pull request from `gh`, in a trusted project only: `git::pr`) and hands them
//! back, as the GPUI app's `window/title/extras.rs` does.

use std::path::PathBuf;

use crate::git::pr::{Branch, PullRequest};
use crate::view::layout::title_parts::TitlePart;

use super::event::Event;
use super::model::WorkspaceModel;

impl WorkspaceModel {
    /// The terminal whose ports the row would show now: the selected one, while the row has the part.
    pub fn ports_wanted(&self) -> Option<i64> {
        self.cfg.title_row.shows(TitlePart::Ports).then_some(self.selected).flatten()
    }

    /// The branches whose pull requests the row may show: each terminal's, while the row has the part
    /// (the sidebar's folder bands show theirs too), once per branch.
    pub fn pull_request_branches(&self) -> Vec<Branch> {
        if !self.cfg.title_row.shows(TitlePart::PullRequest) {
            return Vec::new();
        }
        let mut branches: Vec<Branch> = Vec::new();
        for branch in self.sessions.iter().filter_map(|s| branch_of(s.facts.repo.clone(), &s.facts.branch)) {
            if !branches.contains(&branch) {
                branches.push(branch);
            }
        }
        branches
    }

    /// The first terminal on `branch`: where a pull request's finished checks are told.
    pub fn first_on_branch(&self, branch: &Branch) -> Option<i64> {
        self.sessions
            .iter()
            .find(|s| branch_of(s.facts.repo.clone(), &s.facts.branch).as_ref() == Some(branch))
            .map(|s| s.id)
    }

    /// The ports terminal `id`'s programs listen on, lowest first, as the last look found them.
    pub fn set_ports(&mut self, id: i64, ports: Vec<u16>) -> Vec<Event> {
        let before = self.ports.get(&id).cloned().unwrap_or_default();
        if self.index_of(id).is_some() && before != ports {
            self.ports.insert(id, ports);
            self.emit(Event::Chrome);
        }
        self.out.take()
    }

    /// What `gh` said of `branch`: its pull request, or none.
    pub fn set_pull_request(&mut self, branch: Branch, found: Option<PullRequest>) -> Vec<Event> {
        if self.pull_requests.get(&branch) != found.as_ref() {
            match found {
                Some(pull_request) => self.pull_requests.insert(branch, pull_request),
                None => self.pull_requests.remove(&branch),
            };
            self.emit(Event::Chrome);
        }
        self.out.take()
    }

    /// The pull request of terminal `id`'s branch, when one is known.
    pub(super) fn pull_request_of(&self, id: i64) -> Option<&PullRequest> {
        let facts = self.facts(id)?;
        self.pull_requests.get(&branch_of(facts.repo.clone(), &facts.branch)?)
    }
}

fn branch_of(root: Option<PathBuf>, name: &Option<String>) -> Option<Branch> {
    Some(Branch { root: root?, name: name.clone()? })
}
