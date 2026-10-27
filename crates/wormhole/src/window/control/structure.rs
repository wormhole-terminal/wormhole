//! Changing a window from outside it: splitting a terminal, selecting one, naming one, asking to close one.
//! `app::control::structure` finds the window and calls these, after the switch, the caps and the
//! budgets, which are core's (`wormhole_core::ipc::control::structure`).
//!
//! Nothing here takes the keyboard from the person unless it was asked to: a split leaves the selection
//! and the keys where they are (`select` moves them, inside the window), `focus` selects without moving the
//! keys, and only `focus --activate` gives them to the terminal and brings the window forward.

use gpui_kit::*;
use wormhole_core::ipc::control::{CloseAsked, Error, ErrorCode, Focused, Reason, Renamed, Side, SplitMade};
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::view::panes::{DropError, PaneId, TabPanes, Zone};

use crate::app_state::app;
use crate::window::Workspace;
use crate::window::windows::CloseOrigin;

/// A pane id no terminal has, to ask the layout whether a join would fit before a terminal is made.
const NOT_YET: PaneId = PaneId::MIN;

/// The side of the target's pane a new terminal goes, as a drop zone.
fn zone_of(side: Side) -> Zone {
    match side {
        Side::Right => Zone::Right,
        Side::Down => Zone::Bottom,
        Side::Left => Zone::Left,
        Side::Up => Zone::Top,
    }
}

fn not_found(target: i64) -> Error {
    Error::new(ErrorCode::NotFound, format!("no terminal {target}"))
}

/// What a `split` is asked to make.
pub(crate) struct SplitPlan<'a> {
    pub side: Side,
    /// The new pane's share of the split, in percent.
    pub size: Option<u8>,
    /// Where its shell starts, else where the target's is.
    pub folder: Option<String>,
    /// Typed into its shell and run.
    pub command: Option<&'a str>,
    /// Select it and give it the window's keyboard.
    pub select: bool,
}

impl Workspace {
    /// A new terminal beside terminal `target`, joined into its split (made if it stood alone): in
    /// the target's folder and with its shell profile, as ⌘D makes one, but for any terminal of the window
    /// and without moving the selection or the keyboard unless the plan says. Refused with nothing made
    /// when the window has no room for another pane (`no_room`): the room is the panes' area as the last
    /// frame measured it, the same for every split of the window.
    pub(crate) fn control_split(
        &mut self,
        target: i64,
        plan: SplitPlan<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<SplitMade, Error> {
        let SplitPlan { side, size, folder, command, select } = plan;
        self.control_split_fits(target, side)?;
        let target_ix = self.find_session(target).ok_or_else(|| not_found(target))?;
        let zone = zone_of(side);
        let room = self.pane_room();
        // Whether the target's split is what is on screen now: only then does a new pane open over it.
        let shown = self.on_screen(target_ix);
        let before = self.split_of(target_ix).map(|split| split.layout.clone());
        // Where the keyboard stays in the target's split (the layout moves it to the pane that joined,
        // which is the person's to do with `select`).
        let kept = before.as_ref().map_or(target, |layout| layout.focused());
        let mut layout = before.clone().unwrap_or_else(|| TabPanes::single(target));

        let cwd = folder.or_else(|| self.terminal_cwd(target, cx).map(|p| p.to_string_lossy().into_owned()));
        let id = self.alloc_id(cx);
        let mut record = SessionRecord::new(id, None, cwd.clone());
        record.shell = self.sessions[target_ix].pane.shell.clone();
        let ix = self.add_session(Some(record), window, cx);
        if layout.join(target, zone, id, room).is_err() {
            // Checked above; if the layout changed its mind, a terminal made for nothing goes again.
            self.drop_session(ix, cx);
            return Err(Error::new(ErrorCode::Failed, "the split could not be made"));
        }
        let applied = size
            .and_then(|percent| layout.set_share(id, target, f32::from(percent), room))
            .map(|percent| percent.round().clamp(1.0, 99.0) as u8);
        if !select {
            layout.focus(kept);
        }
        self.install_layout(target_ix, ix, layout, cx);
        if let Some(command) = command {
            self.control_run_first(id, command, cx);
        }
        if select {
            if let Some(at) = self.find_session(id) {
                self.activate(at, window, cx);
            }
            self.begin_opening(before.filter(|_| shown), id, cx);
        }
        cx.notify();
        let split = self.find_session(id).and_then(|at| self.sessions[at].split).unwrap_or_default();
        Ok(SplitMade { window: self.window_id, split, terminal: id, folder: cwd, size: applied })
    }

    /// Whether terminal `target` can be split toward `side` with the room the window has, asked before
    /// anything is made (and before a budget is spent): `no_room` when either pane would end up under the
    /// minimum, which is the panes' area as the last frame measured it.
    pub(crate) fn control_split_fits(&self, target: i64, side: Side) -> Result<(), Error> {
        let target_ix = self.find_session(target).ok_or_else(|| not_found(target))?;
        let layout =
            self.split_of(target_ix).map_or_else(|| TabPanes::single(target), |split| split.layout.clone());
        layout.can_join(target, zone_of(side), NOT_YET, self.pane_room()).map_err(|refused| match refused {
            DropError::TooSmall => Error::new(
                ErrorCode::Refused,
                format!("window {} has no room to split terminal {target} {}", self.window_id, side.word()),
            )
            .because(Reason::NoRoom),
            DropError::Unknown | DropError::Unchanged => not_found(target),
        })
    }

    /// Select terminal `tab`: its row, and its split shown with it. The keyboard stays where it was, even
    /// in the key window; only `activate` gives it to the terminal and brings the window forward, Wormhole
    /// with it, as far as background mode allows.
    pub(crate) fn control_focus(
        &mut self,
        tab: i64,
        activate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Focused> {
        let ix = self.find_session(tab)?;
        if activate {
            self.activate(ix, window, cx);
            crate::background::activate(cx);
            crate::background::activate_window(window, cx);
        } else {
            self.select(ix, cx);
            self.sync_resize_increments(window, cx);
            cx.notify();
        }
        Some(Focused { window: self.window_id, terminal: tab })
    }

    /// Name terminal `tab` (the sidebar's rename), or with no name call it after what runs in it again.
    pub(crate) fn control_rename(
        &mut self,
        tab: i64,
        name: Option<String>,
        cx: &mut Context<Self>,
    ) -> Option<Renamed> {
        let ix = self.find_session(tab)?;
        self.set_custom_name(ix, name, cx);
        let session = &self.sessions[ix];
        Some(Renamed {
            terminal: tab,
            name: session.name().to_string(),
            custom: session.custom_name.is_some(),
        })
    }

    /// Whether a question is up in this window that another would pile onto: a terminal's, the window's
    /// own or the app's about quitting. (Not the update's restart question nor the worktree dialog, which
    /// are asked from elsewhere and do not set a flag of the window's.)
    pub(crate) fn question_is_open(&self, cx: &App) -> bool {
        self.closing_session_prompt || self.closing_prompt || app(cx).quit_prompt == Some(self.window_id)
    }

    /// `question_open` while one is up, asked before a budget is spent.
    pub(crate) fn control_close_check(&self, tab: i64, cx: &App) -> Result<(), Error> {
        if self.find_session(tab).is_none() {
            return Err(not_found(tab));
        }
        if self.question_is_open(cx) {
            return Err(Error::new(
                ErrorCode::Refused,
                format!("a question is already up in window {}: it has to be answered first", self.window_id),
            )
            .because(Reason::QuestionOpen));
        }
        Ok(())
    }

    /// Put the question before closing terminal `tab` in front of the person, always (also when nothing is
    /// running in it), saying that the command line asked. Nothing closes until they answer;
    /// `terminal_closed` follows if they say yes.
    pub(crate) fn control_ask_close(
        &mut self,
        tab: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<CloseAsked, Error> {
        self.control_close_check(tab, cx)?;
        let ix = self.find_session(tab).ok_or_else(|| not_found(tab))?;
        self.ask_close_session(ix, CloseOrigin::Control, window, cx);
        Ok(CloseAsked { asked: true, window: self.window_id, terminal: tab })
    }
}
