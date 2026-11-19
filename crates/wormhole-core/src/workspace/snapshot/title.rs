//! The terminal's title row as data: which parts it shows and in what order, which of them have
//! something to say now, where the chevrons go, the path as it can be drawn at each width and the
//! folders its menu lists. A native title row draws these, measures the text it was given and asks
//! core's `layout::title_row::fit_row` how much of it fits; it keeps no copy of the rules.

use std::path::Path;

use crate::git::pr::PullRequest;
use crate::view::layout::path_menu;
use crate::view::layout::title_parts::{TitlePart, chevrons};
use crate::view::layout::title_row::path_steps;
use crate::view::sidebar::RowState;
use crate::view::sidebar::wording::display_path;

use super::super::model::WorkspaceModel;
use super::super::tile::{TileGlyph, glyph_of};
use super::AgentInfo;

/// A part of the row, in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitleSlot {
    pub part: TitlePart,
    /// The model has something for it (a branch needs a repository, Open in… a folder, ports a
    /// listener, the pull request one that `gh` found). The gauge is the shell's own: never present.
    pub present: bool,
    /// A chevron goes before it, as it joins the crumb before it.
    pub chevron_before: bool,
}

/// The terminal the row is about: the selected one.
#[derive(Clone, Debug, PartialEq)]
pub struct TitleTerminal {
    pub id: i64,
    pub name: String,
    pub agent: Option<AgentInfo>,
    pub glyph: TileGlyph,
    pub state: RowState,
    /// The folder as the list writes it (`~/Projects/app`), and as a full path.
    pub folder: Option<String>,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    /// In a repository with no branch (a detached HEAD): the row says "detached".
    pub detached: bool,
    /// For a submodule, whose it is ("in terminal").
    pub superproject: Option<String>,
    pub message: Option<String>,
}

/// One way of drawing the path: its start and kept parents in the regular weight, its last folder
/// in the medium one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathStep {
    pub parents: String,
    pub last: String,
}

/// A line of the path menu: the folder, then each parent down to the volume.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathFolder {
    pub path: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TitleSnapshot {
    pub revision: u64,
    pub terminal: Option<TitleTerminal>,
    /// The parts before the row's flexible space, and after it.
    pub left: Vec<TitleSlot>,
    pub right: Vec<TitleSlot>,
    /// The parts the person hid, in the default order (the row's customise menu).
    pub hidden: Vec<TitlePart>,
    /// The ways to draw the folder, the widest first: whole, then losing its middle down to
    /// `~/…/last`. Measure each and give `fit_row` the widths.
    pub path_steps: Vec<PathStep>,
    /// The folder's menu (`boot_volume` names the top of the tree).
    pub path_menu: Vec<PathFolder>,
    /// The ports the selected terminal's programs listen on, lowest first.
    pub ports: Vec<u16>,
    /// The pull request of its branch, and how VoiceOver says it (`PullRequest::words`).
    pub pull_request: Option<PullRequest>,
    pub pull_request_words: Option<String>,
}

impl WorkspaceModel {
    /// The title row of the selected terminal. `boot_volume` is what the system calls the startup
    /// volume ("Macintosh HD"), the top of the path menu.
    pub fn title_snapshot(&self, boot_volume: &str) -> TitleSnapshot {
        let revision = self.revisions().terminals;
        let (left, right) = self.chrome_row_ends();
        let hidden = self.cfg.title_row.hidden().collect();
        let Some(session) = self.selected_index().map(|ix| &self.sessions[ix]) else {
            let slots = |parts: &[TitlePart]| self.slots(parts, |_| false);
            return TitleSnapshot {
                revision,
                terminal: None,
                left: slots(&left),
                right: slots(&right),
                hidden,
                path_steps: Vec::new(),
                path_menu: Vec::new(),
                ports: Vec::new(),
                pull_request: None,
                pull_request_words: None,
            };
        };
        let facts = &session.facts;
        let folder = facts.cwd.as_deref().map(|cwd| display_path(cwd, self.cfg.home.as_deref()));
        let ports = self.ports.get(&session.id).cloned().unwrap_or_default();
        let pull_request = self.pull_request_of(session.id).cloned();
        let present = |part: TitlePart| match part {
            TitlePart::Folder | TitlePart::OpenIn => facts.cwd.is_some(),
            TitlePart::Branch => facts.branch.is_some() || facts.repo.is_some(),
            TitlePart::BranchStatus => facts.branch.is_some(),
            TitlePart::Terminal | TitlePart::Splits => true,
            TitlePart::TerminalStatus => facts.told() || facts.command.is_some(),
            TitlePart::Ports => !ports.is_empty(),
            TitlePart::PullRequest => pull_request.is_some(),
            TitlePart::AgentGauge | TitlePart::Space => false,
        };
        TitleSnapshot {
            revision,
            terminal: Some(TitleTerminal {
                id: session.id,
                // What runs in it, not the person's name for it (the GPUI row's last chip is `Pane::label`).
                name: session.label(),
                agent: facts.agent.map(super::sidebar::agent_info),
                glyph: glyph_of(facts),
                state: facts.row_state(),
                folder: folder.clone(),
                cwd: facts.cwd.as_ref().map(|cwd| cwd.to_string_lossy().into_owned()),
                branch: facts.branch.clone(),
                detached: facts.branch.is_none() && facts.repo.is_some(),
                superproject: facts.superproject.clone(),
                message: facts.message.clone(),
            }),
            left: self.slots(&left, present),
            right: self.slots(&right, present),
            hidden,
            path_steps: folder
                .as_deref()
                .map(|shown| {
                    // Unbounded and unmeasured: the shell measures each text in its own font.
                    path_steps(shown, f32::INFINITY, |_, text| text.chars().count() as f32)
                        .into_iter()
                        .map(|step| PathStep { parents: step.text.parents, last: step.text.last })
                        .collect()
                })
                .unwrap_or_default(),
            path_menu: facts.cwd.as_deref().map(|cwd| path_menu_of(cwd, boot_volume)).unwrap_or_default(),
            pull_request_words: pull_request.as_ref().map(PullRequest::words),
            pull_request,
            ports: ports.clone(),
        }
    }

    fn chrome_row_ends(&self) -> (Vec<TitlePart>, Vec<TitlePart>) {
        let (left, right) = self.cfg.title_row.ends();
        (left.to_vec(), right.to_vec())
    }

    /// One end of the row as slots: the chevrons join the crumbs that are drawn.
    fn slots(&self, parts: &[TitlePart], present: impl Fn(TitlePart) -> bool) -> Vec<TitleSlot> {
        let drawn: Vec<TitlePart> = parts.iter().copied().filter(|&part| present(part)).collect();
        let mut marks = chevrons(&drawn).into_iter();
        parts
            .iter()
            .map(|&part| {
                let present = present(part);
                TitleSlot { part, present, chevron_before: present && marks.next().unwrap_or(false) }
            })
            .collect()
    }
}

fn path_menu_of(cwd: &Path, boot_volume: &str) -> Vec<PathFolder> {
    path_menu::chain(cwd, boot_volume)
        .into_iter()
        .map(|folder| PathFolder { path: folder.path.to_string_lossy().into_owned(), name: folder.name })
        .collect()
}
