//! Renaming a row of the tree in place (Rename… in its menu), the one thing the Files tab writes.
//!
//! The row's name becomes a text field (the app's own, as the sidebar's rename uses) with the name
//! selected but not its extension, as Finder does; ↩ commits, Esc or a click elsewhere cancels. The
//! rename itself is core's [`rename_within`](wormhole_core::storage::files::rename_within), run in the
//! background: it refuses a name that can't be one and anything that exists, and goes through no link. When
//! it is refused the field stays open with what was typed and the notice chip says why (VoiceOver too);
//! when it works the selection and an open viewer follow the new name.
//!
//! Only on the person's action: nothing here is called by a timer or by what a repository holds.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::storage::files::{RenameError, rename_within};
use wormhole_core::view::file_tree::names::stem_len;
use wormhole_core::view::file_tree::{Row, RowKind};
use wormhole_core::view::notice::Failed;
use wormhole_text::{Key, keys, text};

use super::Files;
use crate::platform::a11y;
use crate::window::feature::FeatureEvent;

/// The row being renamed, and its name field.
pub(super) struct Renaming {
    session: i64,
    /// The row's path below the root, as it was when the field opened.
    path: PathBuf,
    pub(super) input: Entity<InputState>,
    /// The rename is being made: a click away (the field loses the keys) must not cancel it now.
    busy: bool,
    _events: Subscription,
}

/// What the notice chip and VoiceOver say of a refused rename.
fn failure(error: &RenameError) -> (Failed, Key) {
    match error {
        RenameError::Name(_) => (Failed::RenameName, keys::NOTICE_FAILED_RENAME_NAME),
        RenameError::Exists => (Failed::RenameExists, keys::NOTICE_FAILED_RENAME_EXISTS),
        RenameError::Denied => (Failed::RenameDenied, keys::NOTICE_FAILED_RENAME_DENIED),
        RenameError::Missing | RenameError::Outside => (Failed::RenameGone, keys::NOTICE_FAILED_RENAME_GONE),
        RenameError::Failed(_) => (Failed::RenameFailed, keys::NOTICE_FAILED_RENAME_FAILED),
    }
}

impl Renaming {
    /// The row's path and its field, for the tree to draw.
    pub(super) fn target(&self) -> (PathBuf, Entity<InputState>) {
        (self.path.clone(), self.input.clone())
    }

    /// The terminal whose tree the row is in.
    pub(super) fn session(&self) -> i64 {
        self.session
    }
}

impl Files {
    /// Turns the name of `row` into a field. Only an entry of the tree can be renamed (a file, a folder or
    /// a link: the link itself), never the root.
    pub(super) fn start_rename(
        &mut self,
        session: i64,
        row: &Row,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let renamable = matches!(row.kind, RowKind::File | RowKind::Folder { .. } | RowKind::Link);
        if !renamable
            || row.path.as_os_str().is_empty()
            || self.of.get(&session).is_none_or(|of| of.root.is_none())
        {
            return;
        }
        let name = row.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let selected = stem_len(&name, row.is_folder());
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.commit_rename(window, cx),
            InputEvent::Blur => this.cancel_rename(cx),
            _ => {}
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.set_selected_range(0..selected, cx);
        });
        let before = self.renaming.replace(Renaming {
            session,
            path: row.path.clone(),
            input,
            busy: false,
            _events: events,
        });
        // A field that was open is replaced: the window keeps hearing that a field has the keys.
        if before.is_none() {
            cx.emit(FeatureEvent::Editing(true));
        }
        cx.notify();
    }

    /// The row being renamed in `session`'s tree and what is typed in its field, for devctl.
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn renaming_of(&self, session: i64, cx: &App) -> Option<String> {
        let renaming = self.renaming.as_ref().filter(|r| r.session == session)?;
        Some(format!("{} -> {:?}", renaming.path.display(), renaming.input.read(cx).value()))
    }

    /// Types `name` in the open field in place of what it holds, for devctl.
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn type_rename(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(renaming) = &self.renaming {
            renaming.input.update(cx, |input, cx| input.set_value(name.to_string(), window, cx));
        }
    }

    /// Esc, or the keys going elsewhere: the old name stays.
    pub(super) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        // A rename that is already being made is not undone by the field losing the keys.
        if self.renaming.as_ref().is_some_and(|r| r.busy) {
            return;
        }
        self.end_rename(cx);
    }

    fn end_rename(&mut self, cx: &mut Context<Self>) {
        if self.renaming.take().is_some() {
            cx.emit(FeatureEvent::Editing(false));
            cx.notify();
        }
    }

    /// ↩: renames to what was typed. The same name again, or an empty one, only closes the field.
    pub(super) fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.as_mut().filter(|r| !r.busy) else { return };
        let typed = renaming.input.read(cx).value().to_string();
        let (session, from) = (renaming.session, renaming.path.clone());
        let unchanged = from.file_name().is_some_and(|old| old.to_string_lossy() == typed);
        if unchanged {
            self.end_rename(cx);
            return;
        }
        let Some(root) = self.of.get(&session).and_then(|of| of.root.clone()) else {
            self.end_rename(cx);
            return;
        };
        renaming.busy = true;
        let handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            let asked = (root, from.clone());
            let result = cx
                .background_spawn(async move { rename_within(&asked.0, &asked.1, &OsString::from(typed)) })
                .await;
            this.update(cx, |files, cx| files.renamed(session, &from, result, handle, cx)).ok();
        })
        .detach();
    }

    /// The rename came back. A refusal keeps the field open for another try and says why; success moves the
    /// selection and an open viewer to the new name and reads the folder again.
    fn renamed(
        &mut self,
        session: i64,
        from: &Path,
        result: Result<PathBuf, RenameError>,
        window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        match result {
            Err(error) => {
                if let Some(renaming) =
                    self.renaming.as_mut().filter(|r| r.session == session && r.path == from)
                {
                    renaming.busy = false;
                }
                let (notice, words) = failure(&error);
                tracing::info!("not renaming {}: {error}", from.display());
                crate::notice::post_to(window, notice, None, cx);
                a11y::announce(text(words), false);
                cx.notify();
            }
            Ok(to) => {
                if self.renaming.as_ref().is_some_and(|r| r.session == session && r.path == from) {
                    self.end_rename(cx);
                }
                self.followed_rename(session, from, &to, cx);
            }
        }
    }

    /// The selection, an open viewer and the folder's listing follow a rename of `from` to `to`.
    fn followed_rename(&mut self, session: i64, from: &Path, to: &Path, cx: &mut Context<Self>) {
        let parent = to.parent().map(Path::to_path_buf).unwrap_or_default();
        let of = self.of_mut(session);
        if of.selected.as_deref().is_some_and(|selected| selected.starts_with(from)) {
            of.selected = of
                .selected
                .take()
                .and_then(|selected| selected.strip_prefix(from).ok().map(|rest| to.join(rest)));
        }
        // The folder changed: it is listed again at once, not at the next look.
        of.stamps.remove(&parent);
        of.stale = true;
        self.viewer_renamed(session, from, to, cx);
        self.read(session, parent, cx);
        let name = to.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        a11y::announce(&keys::files_spoken_renamed(&name), false);
        cx.notify();
    }
}

/// The field that stands in the name's place on the row being renamed: a click in it is its own, and
/// Esc drops what was typed.
pub(super) fn field(this: WeakEntity<Files>, input: &Entity<InputState>) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .capture_key_down(move |event: &KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                this.update(cx, |files, cx| files.cancel_rename(cx)).ok();
            }
        })
        .child(Input::new(input).xsmall().aria_label(text(keys::FILES_SPOKEN_RENAME_FIELD)))
        .into_any_element()
}
