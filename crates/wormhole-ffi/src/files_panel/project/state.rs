//! The Project tab's state, made from a snapshot of the folder, the person's switches and the trust
//! store: the wording is the app's (`features/project/panel.rs`), written once here.

use wormhole_core::project::{
    Action, ActionKind, Feature, ProjectSnapshot, TrustStatus, dir_name_at, trust_applies,
};

use super::hub::Hub;
use super::records::{
    ProjectActionRecord, ProjectFeatureRecord, ProjectPromptRecord, ProjectStateRecord, TrustLineRecord,
};

/// The tab's state for `snapshot`; remembers what it showed, which approving trusts.
pub(super) fn build(hub: &mut Hub, snapshot: &ProjectSnapshot) -> ProjectStateRecord {
    let root = snapshot.root().to_path_buf();
    let status = hub.trust.status_of(snapshot);
    let request = snapshot.trust_request();
    match &request {
        Some(request) => hub.shown.insert(root.clone(), request.clone()),
        None => hub.shown.remove(&root),
    };
    let commands: Vec<TrustLineRecord> =
        request.as_ref().map(|r| r.commands.iter().map(TrustLineRecord::from).collect()).unwrap_or_default();
    let needing = snapshot.needing_trust();
    // Notes sync waits for trust too (its notes.md is repository content), so the notice is there to say
    // so, though nothing else in the project asks for it.
    let notes_wait =
        status != TrustStatus::Trusted && hub.prefs.notes_sync(&root, snapshot.has(Feature::Notes));
    let applies = match (trust_applies(&needing), notes_wait) {
        (applies, false) => applies,
        (applies, true) if applies.is_empty() => "its notes.md sync".to_string(),
        (applies, true) => format!("{applies} and its notes.md sync"),
    };
    let asks_for_trust = request.is_some() && (!commands.is_empty() || !needing.is_empty() || notes_wait);

    let features = Feature::ALL
        .into_iter()
        .filter(|feature| snapshot.has(*feature) || *feature == Feature::Notes)
        .map(|feature| {
            let present = snapshot.has(feature);
            let on = match feature {
                // Notes sync where the file is, not wherever the switch defaults to.
                Feature::Notes => hub.prefs.notes_sync(&root, present),
                _ => hub.prefs.enabled(&root, feature),
            };
            let (hint, warns) = feature_hint(snapshot, status, feature);
            ProjectFeatureRecord { key: feature.key().to_string(), on, hint, warns }
        })
        .collect();

    let actions = if hub.enabled(snapshot, Feature::Actions) {
        snapshot.actions().iter().map(action_record).collect()
    } else {
        Vec::new()
    };
    let prompts = if hub.enabled(snapshot, Feature::Prompts) {
        snapshot
            .prompts()
            .iter()
            .map(|p| ProjectPromptRecord {
                id: p.id.clone(),
                title: p.title.clone(),
                detail: p.description.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let layout = match snapshot.layout() {
        Some(layout) if hub.enabled(snapshot, Feature::Layout) => {
            layout.sessions.iter().map(|session| session.name.clone()).collect()
        }
        _ => Vec::new(),
    };

    let notes_error = hub.notes_errors.get(&root).map(|err| format!("Notes sync: {err}"));
    let errors = hub
        .error
        .iter()
        .cloned()
        .chain(snapshot.errors().map(|err| err.to_string()))
        .chain(notes_error)
        .collect();
    ProjectStateRecord {
        dir: dir_name_at(&root),
        root: crate::files_panel::names::path_key(&root),
        status: status.into(),
        commands,
        applies,
        asks_for_trust,
        features,
        actions,
        prompts,
        layout,
        errors,
        warnings: snapshot.warnings().iter().map(|warning| warning.to_string()).collect(),
    }
}

/// The line under a feature's switch, and whether it is a warning: what is held back by trust is, a
/// file that sets nothing yet is not. Notes have their own line: the switch is how the file gets made.
fn feature_hint(snapshot: &ProjectSnapshot, status: TrustStatus, feature: Feature) -> (String, bool) {
    if feature == Feature::Notes {
        if status != TrustStatus::Trusted {
            // The sync reads and writes a file that comes with the repository.
            return ("Syncs once the project is trusted".to_string(), true);
        }
        let hint =
            if snapshot.has(feature) { feature.description() } else { "On creates notes.md and syncs notes" };
        return (hint.to_string(), false);
    }
    let standing = snapshot.standing(feature, status);
    (standing.hint(feature).unwrap_or_else(|| feature.description().to_string()), standing.warns())
}

fn action_record(action: &Action) -> ProjectActionRecord {
    ProjectActionRecord {
        id: action.id.clone(),
        title: action.title.clone(),
        detail: action_detail(action),
        key: action.key_spec().map(|key| key.glyphs()),
    }
}

/// An action's second line: its description, or else what it does.
fn action_detail(action: &Action) -> Option<String> {
    if let Some(description) = &action.description {
        return Some(description.clone());
    }
    match &action.kind {
        ActionKind::Run { command, .. } => Some(command.clone()),
        ActionKind::Prompt { file: Some(file), .. } => Some(format!("prompts/{file}")),
        ActionKind::Prompt { text, .. } => text.as_ref().and_then(|t| t.lines().next()).map(str::to_string),
        ActionKind::Open { url } => Some(url.clone()),
        ActionKind::Chain { steps } => Some(format!("{} steps", steps.len())),
    }
}
