//! What a typed query finds: every group that finds something, in the palette's group order.

use wormhole_core::config::settings_index::{self, Query};
use wormhole_core::view::search::{self, Candidate, Group, Ranked, Scope};

use super::items::{Item, Row, label};
use super::snapshot::Snapshot;

/// A query: every group that finds something (or the scope's), in the palette's group order.
pub(super) fn found(snapshot: &Snapshot, scope: Scope, query: &str) -> Vec<Row> {
    let wanted = |group: Group| scope.group().is_none_or(|g| g == group);
    let mut groups: Vec<(Group, Vec<Ranked>)> = Vec::new();
    let mut items: Vec<(Group, Vec<Item>)> = Vec::new();
    // The shell entries' labels are made here (a candidate borrows its words).
    let shell_words: Vec<(Item, String, &str)> = snapshot
        .shell_items()
        .filter_map(|item| snapshot.shell_words(&item).map(|(label, line)| (item, label, line)))
        .collect();
    let profile_words: Vec<(Item, String)> = snapshot
        .profile_items()
        .into_iter()
        .filter_map(|item| {
            let Item::Profile(_, id) = &item else { return None };
            let label = item.profile_label(snapshot.profile(id)?)?;
            Some((item, label))
        })
        .collect();
    if wanted(Group::Settings) {
        // The settings index scores its own rows (label first, then config key, description…)
        // and lists them page by page, as its scope does; everything searched shows only a few
        // of them, so there the best match comes first.
        // Drop hits in a section this build hides (a Preview section whose flag is off), so a
        // release build never lists Project folders or Feature flags rows here.
        let hits: Vec<_> = settings_index::search(&Query::new(query))
            .into_iter()
            .filter(|hit| !snapshot.hidden_sections.contains(&hit.section))
            .collect();
        let mut ranked: Vec<Ranked> = hits
            .iter()
            .enumerate()
            .map(|(index, hit)| Ranked {
                index,
                found: search::Match {
                    score: hit.score as i32,
                    ranges: hit.label_range.iter().cloned().collect(),
                },
            })
            .collect();
        if scope == Scope::All {
            ranked.sort_by(|a, b| b.found.score.cmp(&a.found.score).then(a.index.cmp(&b.index)));
        }
        items.push((Group::Settings, hits.iter().map(|hit| Item::Setting(hit.entry.id)).collect()));
        groups.push((Group::Settings, ranked));
    }
    if wanted(Group::Actions) {
        let mut list: Vec<Item> = snapshot.actions.iter().map(|a| Item::Action(a.id)).collect();
        let mut candidates: Vec<Candidate> =
            snapshot.actions.iter().map(|a| Candidate::new(&a.label)).collect();
        if let Some(project) = &snapshot.project {
            list.extend(project.actions.iter().map(|a| Item::ProjectAction(a.id.clone())));
            candidates.extend(project.actions.iter().map(|a| match &a.command {
                Some(command) => Candidate::new(&a.title).with(command),
                None => Candidate::new(&a.title),
            }));
        }
        for (item, label, command_line) in &shell_words {
            list.push(item.clone());
            candidates.push(Candidate::new(label).with(command_line));
        }
        for item in &profile_words {
            list.push(item.0.clone());
            candidates.push(Candidate::new(&item.1));
        }
        groups.push((Group::Actions, search::rank(query, candidates)));
        items.push((Group::Actions, list));
    }
    if wanted(Group::Terminals) {
        let candidates = snapshot.terminals.iter().map(|t| {
            let terminal = &t.terminal;
            let mut candidate = Candidate::new(&terminal.name);
            for text in
                [&terminal.folder, &terminal.branch, &terminal.command, &terminal.who, &terminal.message]
                    .into_iter()
                    .flatten()
            {
                candidate = candidate.with(text);
            }
            candidate
        });
        groups.push((Group::Terminals, search::rank(query, candidates)));
        items.push((
            Group::Terminals,
            snapshot.terminals.iter().map(|t| Item::Terminal(t.terminal.tab)).collect(),
        ));
    }
    if wanted(Group::Folders) {
        let candidates = snapshot.folders.iter().map(|f| Candidate::new(&f.name).with(&f.parent));
        groups.push((Group::Folders, search::rank(query, candidates)));
        items.push((Group::Folders, snapshot.folders.iter().map(|f| Item::Folder(f.path.clone())).collect()));
    }
    if wanted(Group::Commits) {
        let candidates =
            snapshot.commits.iter().map(|c| Candidate::new(&c.subject).with(&c.short).with(&c.author));
        groups.push((Group::Commits, search::rank(query, candidates)));
        items.push((Group::Commits, snapshot.commits.iter().map(|c| Item::Commit(c.sha.clone())).collect()));
    }
    let mut rows = Vec::new();
    for section in search::sections(scope, groups) {
        let Some((_, list)) = items.iter().find(|(group, _)| *group == section.group) else { continue };
        rows.push(label(section.group.title(), Some(section.total)));
        rows.extend(section.shown.into_iter().filter_map(|ranked| {
            let item = list.get(ranked.index)?.clone();
            Some(Row::Result { item, marks: ranked.found.ranges })
        }));
    }
    rows
}
