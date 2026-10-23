//! Two-way sync of each session's notes and tasks with its project's `notes.md` (the merge itself
//! is in core: `wormhole_core::project::plan_sync`).

use gpui_kit::*;
use wormhole_core::project::{self as proj, Feature, NotesBase, NotesDoc, NotesFile};

use super::{hub, hub_mut, project_folders_on};
use crate::window::Workspace;
use crate::window::features::notes::Notes;

impl Workspace {
    /// Two-way sync between each session's notes/tasks and its project's `notes.md`, as a three-way
    /// merge against the last state both agreed on (kept per session and project across restarts).
    pub(in crate::window) fn sync_project_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // It runs more often than `refresh_projects`, so the `notes_root`s may not be cleared yet
        // when the project-folder flag goes off: check the flag itself before touching notes.md.
        if !project_folders_on(cx) || !crate::window::features::notes::on(cx) {
            return;
        }
        let Some(notes) = self.feature::<Notes>() else { return };
        for ix in 0..self.sessions.len() {
            let session_id = self.sessions[ix].id;
            let Some(root) = self.sessions[ix].notes_root.clone() else { continue };
            let hub = hub(cx);
            let Some(project) = hub.projects.get(&root) else { continue };
            // Syncs when switched on and the project has a notes.md, or when the user
            // explicitly opted in for this project (which creates the file). The Project tab's
            // switch asks the same question, so it shows on only when this runs.
            if !hub.prefs.notes_sync(&root, project.folder.has(Feature::Notes)) {
                // Switched off: switching on again merges both sides afresh.
                self.sessions[ix].notes_base = None;
                hub_mut(cx).notes_bases.remove(session_id);
                continue;
            }
            let folder = project.folder.clone();
            let local = notes.read(cx).snapshot(session_id, cx);
            let editing = notes.read(cx).being_written(session_id);
            // `notes_base` is this run's copy of the base (empty after a launch); the hub knows
            // which project it belongs to and keeps it for the next launch.
            let session_base = self.sessions[ix].notes_base.clone();
            let hub = hub_mut(cx);
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
            self.sessions[ix].notes_base = Some(plan.base.doc);
        }
        hub_mut(cx).save_notes_bases();
    }
}
