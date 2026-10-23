//! The one place the project feature works with the notes feature, through notes' `api`: two-way sync
//! of each terminal's notes and tasks with its project's `notes.md` (the merge itself is in core:
//! `wormhole_core::project::plan_sync`), and a project prompt copied into a new note.

use gpui_kit::*;
use wormhole_core::config::FeatureId;
use wormhole_core::project::{self as proj, Feature, NotesBase, NotesDoc, NotesFile};
use wormhole_core::storage::paths::ProfileId;

use super::{Project, hub_of, hub_of_mut, project_folders_on};
use crate::features::notes::Notes;
use crate::window::facts::WindowFacts;

/// The window's notes, as the Project tab's button that copies a prompt into a note holds them, with the
/// profile the window is in (whose approvals say whether the prompt may be used). The only way the rest of
/// the project feature gets at the notes feature.
#[derive(Clone)]
pub(super) struct NotesLink {
    notes: Entity<Notes>,
    profile: ProfileId,
}

impl NotesLink {
    /// The window's notes, if it has any.
    pub(super) fn of(facts: &WindowFacts<'_>) -> Option<Self> {
        facts.feature::<Notes>().map(|notes| Self { notes, profile: facts.profile.clone() })
    }
}

impl Project {
    /// Two-way sync between each terminal's notes/tasks and its project's `notes.md`, as a three-way
    /// merge against the last state both agreed on (kept per terminal and project across restarts).
    pub(super) fn sync_notes(
        &mut self,
        facts: &WindowFacts<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // It runs more often than the look for projects, so the terminals' projects may not be cleared yet
        // when the project-folder flag goes off: check the flag itself before touching notes.md.
        if !project_folders_on(facts.profile, cx)
            || !crate::settings::features_in(facts.profile, cx).has(FeatureId::Notes)
        {
            return;
        }
        let Some(notes) = facts.feature::<Notes>() else { return };
        let profile = facts.profile;
        for session in facts.sessions() {
            let session_id = session.id;
            let Some(root) = self.of.get(&session_id).and_then(|of| of.root.clone()) else { continue };
            let Some(project) = hub_of(profile, cx).projects.get(&root) else { continue };
            // Syncs when switched on and the project has a notes.md, or when the user explicitly opted in
            // for this project (which creates the file). The Project tab's switch asks the same question,
            // so it shows on only when this runs.
            if !hub_of(profile, cx).prefs.notes_sync(&root, project.folder.has(Feature::Notes)) {
                // Switched off: switching on again merges both sides afresh.
                if let Some(of) = self.of.get_mut(&session_id) {
                    of.notes_base = None;
                }
                if let Some(hub) = hub_of_mut(profile, cx) {
                    hub.notes_bases.remove(session_id);
                }
                continue;
            }
            let folder = project.folder.clone();
            let local = notes.read(cx).snapshot(session_id, cx);
            let editing = notes.read(cx).being_written(session_id);
            // `notes_base` is this run's copy of the base (empty after a launch); the hub knows which
            // project it belongs to and keeps it for the next launch.
            let session_base = self.of.get(&session_id).and_then(|of| of.notes_base.clone());
            let Some(hub) = hub_of_mut(profile, cx) else { continue };
            let base = hub.notes_bases.get(session_id).map(|saved| NotesBase {
                root: saved.root.clone(),
                doc: session_base.unwrap_or_else(|| saved.doc.clone()),
            });
            let file = hub.notes_files.entry(root.clone()).or_insert_with(|| NotesFile::new(folder));
            let plan = match file.read() {
                (Ok(theirs), _) => {
                    let empty = NotesDoc::default();
                    proj::plan_sync(base.as_ref(), &root, &local, theirs.as_ref().unwrap_or(&empty), editing)
                }
                (Err(err), fresh) => {
                    if fresh {
                        tracing::warn!("notes.md: {err}");
                    }
                    hub.notes_errors.insert(root, err.to_string());
                    continue;
                }
            };
            let Some(plan) = plan else { continue };
            if let Some(doc) = &plan.write
                && let Err(err) = file.write(doc)
            {
                let err = err.to_string();
                if hub.notes_errors.get(&root) != Some(&err) {
                    tracing::warn!("writing notes.md: {err}");
                }
                hub.notes_errors.insert(root, err);
                continue;
            }
            hub.notes_errors.remove(&root);
            hub.notes_bases.set(session_id, plan.base.clone());
            if let Some(doc) = &plan.apply {
                notes.update(cx, |notes, cx| notes.apply(session_id, doc, window, cx));
            }
            if let Some(of) = self.of.get_mut(&session_id) {
                of.notes_base = Some(plan.base.doc);
            }
        }
        if let Some(hub) = hub_of_mut(profile, cx) {
            hub.save_notes_bases();
        }
    }

    /// Copy prompt `id` into a new note on terminal `session`, where the project has Prompts on. `panel` is
    /// the panel's width, for the note's hint.
    pub(super) fn prompt_to_note(
        &mut self,
        session: i64,
        id: &str,
        panel: f32,
        notes: &NotesLink,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(body) = self.enabled_prompt(session, &notes.profile, id, cx) else { return };
        notes.notes.update(cx, |notes, cx| notes.note_from(session, body, panel, window, cx));
    }
}
