//! Keyboard, mouse and scroll input for the terminal.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::keys::keyspec::{KeyMods, KeySpec};
use wormhole_term::Pty;
use wormhole_term::input::{file_drop, keys, mouse};
use wormhole_term::types::LineSize;
use wormhole_term::types::{Column, Line, Modes, Point as GridPoint, Scroll, SelectionType, Side};
use wormhole_term::{Direction, KeySent};

use super::status::TerminalEvent;
use super::{Layout, TerminalView};
use crate::settings::settings_in;

/// What the app may take from a keystroke before any terminal sees it: `true` when it did
/// (a project action's key ran).
pub type KeyClaim = fn(&Keystroke, &mut Window, &mut App) -> bool;

/// Routes keystrokes to the focused terminal. Key bindings are matched before
/// key listeners in GPUI, so terminals take keys through an interceptor
/// (otherwise e.g. Tab would move focus). GPUI runs every interceptor on every
/// keystroke, cloning it for each, so there is one for all terminals; it is also where the
/// app's keys go first (F6, a project's keys), since every interceptor runs whatever the
/// others did.
pub struct TerminalKeys {
    terminals: Vec<WeakEntity<TerminalView>>,
    claim: Option<KeyClaim>,
    _intercept: Subscription,
}

impl Global for TerminalKeys {}

impl TerminalKeys {
    fn ensure(cx: &mut App) {
        if cx.has_global::<Self>() {
            return;
        }
        let intercept = cx.intercept_keystrokes(|event, window, cx| {
            let keys = cx.global::<Self>();
            if let Some(claim) = keys.claim
                && claim(&event.keystroke, window, cx)
            {
                cx.stop_propagation();
                return;
            }
            // App keys a terminal would otherwise send to its program (F6) go to the bindings.
            if keymap::terminal_yields(&keystroke_spec(&event.keystroke)) {
                return;
            }
            let focused = cx.global::<Self>().terminals.iter().find_map(|terminal| {
                let terminal = terminal.upgrade()?;
                terminal.read(cx).focus_handle.is_focused(window).then_some(terminal)
            });
            let Some(terminal) = focused else { return };
            if terminal.update(cx, |this, cx| this.on_keystroke(&event.keystroke, cx)) {
                cx.stop_propagation();
            }
        });
        cx.set_global(Self { terminals: Vec::new(), claim: None, _intercept: intercept });
    }

    pub(super) fn register(terminal: WeakEntity<TerminalView>, cx: &mut App) {
        Self::ensure(cx);
        let keys = cx.global_mut::<Self>();
        keys.terminals.retain(|terminal| terminal.upgrade().is_some());
        keys.terminals.push(terminal);
    }

    /// Let `claim` see every keystroke first, in any window, the focus anywhere.
    pub fn claim_first(claim: KeyClaim, cx: &mut App) {
        Self::ensure(cx);
        cx.global_mut::<Self>().claim = Some(claim);
    }
}

/// A keystroke as a key spec, to look it up in the keymap (`fn` doesn't count).
pub fn keystroke_spec(keystroke: &Keystroke) -> KeySpec {
    let m = &keystroke.modifiers;
    let mods = KeyMods { cmd: m.platform, ctrl: m.control, alt: m.alt, shift: m.shift };
    KeySpec::new(mods, &keystroke.key)
}

/// The ⌘ shortcuts that belong to the terminal; the rest go to the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TerminalCommand {
    Copy,
    Paste,
    SelectAll,
    /// Erase the history and have the shell redraw its prompt.
    Clear,
    /// Bytes for the shell's line editor (⌘⌫ deletes the line, ⌘← / ⌘→ jump).
    Send(&'static [u8]),
    /// ⌘↑ / ⌘↓: the previous or next prompt.
    Prompt(Direction),
}

/// What a focused terminal does with a ⌘ key, by the keymap's table. A key the table doesn't
/// give the terminal is also tried with ⌘ alone (⇧⌘C copies, as it always did), unless it is an
/// app key.
pub(super) fn platform_command(key: &KeySpec) -> Option<TerminalCommand> {
    let action = keymap::terminal_action(key).or_else(|| {
        let app_key = keymap::action_for(key, keymap::Scope::App).is_some();
        let plain = KeySpec::new(KeyMods { cmd: true, ..KeyMods::default() }, &key.key);
        (!app_key).then(|| keymap::terminal_action(&plain)).flatten()
    })?;
    Some(match action {
        ActionId::Copy => TerminalCommand::Copy,
        ActionId::Paste => TerminalCommand::Paste,
        ActionId::SelectAll => TerminalCommand::SelectAll,
        ActionId::ClearBuffer => TerminalCommand::Clear,
        // As Terminal.app and iTerm2: ⌃U, ⌃A and ⌃E for the shell's line editor.
        ActionId::DeleteLine => TerminalCommand::Send(b"\x15"),
        ActionId::LineStart => TerminalCommand::Send(b"\x01"),
        ActionId::LineEnd => TerminalCommand::Send(b"\x05"),
        ActionId::PreviousPrompt => TerminalCommand::Prompt(Direction::Up),
        ActionId::NextPrompt => TerminalCommand::Prompt(Direction::Down),
        // Undo, redo and cut mean nothing in a terminal; the block actions without a key come
        // through the menu as GPUI actions.
        _ => return None,
    })
}

impl TerminalView {
    pub(super) fn on_keystroke(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let m = &keystroke.modifiers;
        if m.platform {
            let Some(command) = platform_command(&keystroke_spec(keystroke)) else { return false };
            self.run_command(command, cx);
            cx.notify();
            return true;
        }

        // Which Option key is down is only asked for when it matters.
        let option_as_meta = m.alt && {
            let (left, right) = crate::platform::keys::option_sides();
            settings_in(&self.profile, cx).option_as_meta.is_meta(left, right)
        };
        let Some(pty) = &self.pty else { return false };
        let input = keys::KeyInput {
            key: &keystroke.key,
            key_char: keystroke.key_char.as_deref(),
            shift: m.shift,
            alt: m.alt,
            control: m.control,
            platform: m.platform,
            option_as_meta,
        };
        match pty.send_key(|mode| keys::encode(&input, mode)) {
            KeySent::Nothing => false,
            KeySent::Sent { view_changed } => {
                self.last_input = Instant::now();
                if self.answered(&keystroke.key, m.control) {
                    self.announce_summary(cx);
                }
                let was_off = self.cursor_off;
                self.hold_cursor();
                self.app_selection.forget();
                // The echo redraws anyway; only a cleared selection or a jump
                // back to the live screen needs a frame of its own.
                if view_changed || was_off {
                    cx.notify();
                }
                true
            }
        }
    }

    pub(super) fn run_command(&mut self, command: TerminalCommand, cx: &mut Context<Self>) {
        match command {
            TerminalCommand::Copy => {
                if let Some(text) = self.selection_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            TerminalCommand::Paste => {
                let clipboard = cx.read_from_clipboard().and_then(|item| item.text());
                // A program that left the mouse to us has no selection of its own any more.
                let cursor_row =
                    self.pty.as_ref().filter(|_| mouse::wants_mouse(self.mode)).and_then(Pty::cursor_row);
                let plan = self.app_selection.paste(clipboard, cursor_row);
                if plan.replace_selection {
                    self.delete_app_selection();
                }
                if let Some(text) = plan.text {
                    self.paste(&text);
                }
            }
            TerminalCommand::SelectAll => {
                if let Some(pty) = &self.pty {
                    pty.select_all();
                }
            }
            TerminalCommand::Clear => {
                if let Some(pty) = &self.pty {
                    pty.clear_history();
                }
                // Ask the shell to redraw a clean prompt.
                self.write_input(b"\x0c".to_vec());
                cx.emit(TerminalEvent::Cleared);
            }
            TerminalCommand::Send(bytes) => self.write_input(bytes.to_vec()),
            TerminalCommand::Prompt(direction) => self.jump_prompt(direction, cx),
        }
    }

    /// Backspace deletes the selection a program that tracks the mouse drew (Claude Code).
    fn delete_app_selection(&mut self) {
        let Some(pty) = &self.pty else { return };
        let backspace = keys::KeyInput { key: "backspace", ..keys::KeyInput::default() };
        pty.send_key(|mode| keys::encode(&backspace, mode));
    }

    /// Files dropped on the pane: insert their paths as if pasted, like Terminal.app. Going
    /// through `paste` keeps agents (Claude Code) treating a dropped image as an attachment.
    pub fn drop_files(&mut self, files: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        let text = file_drop::dropped_text(files.paths());
        if text.is_empty() {
            return;
        }
        self.paste(&text);
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    /// The grid point under `position`, as the last frame showed the grid.
    pub(super) fn grid_point(&self, position: Point<Pixels>) -> Option<(GridPoint, Side)> {
        self.pty.as_ref()?;
        let (col, row, side) = cell_at(&self.layout.get(), position, |row| self.row_size(row))?;
        let offset = self.render_state.borrow().display_offset() as i32;
        Some((GridPoint::new(Line(row as i32 - offset), Column(col)), side))
    }

    /// Viewport cell under the pointer (0-based, ignores scrollback offset).
    pub(super) fn viewport_cell(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        cell_at(&self.layout.get(), position, |row| self.row_size(row)).map(|(col, row, _)| (col, row))
    }

    /// How big viewport `row` was drawn in the last frame (double-width and double-height rows
    /// show half the columns, twice as wide).
    pub(super) fn row_size(&self, row: usize) -> LineSize {
        self.render_state.borrow().snapshot().rows().get(row).map_or(LineSize::Single, |row| row.size())
    }

    /// Mouse events go to the app when it asked for them, unless Shift is held
    /// (Shift forces a local text selection, as in other terminals).
    pub(super) fn report_mouse(
        &mut self,
        button: mouse::MouseButton,
        action: mouse::MouseAction,
        position: Point<Pixels>,
        modifiers: Modifiers,
    ) -> bool {
        let mode = self.mode;
        if self.pty.is_some() && mouse::wants_locator(mode) && !modifiers.shift {
            return self.report_locator(button, action, position);
        }
        if self.pty.is_none() || !mouse::wants_mouse(mode) || modifiers.shift {
            return false;
        }
        let Some((col, row)) = self.viewport_cell(position) else {
            return true;
        };
        // A program that asked for pixels (1016) hears every move; the others only a new cell.
        let pixel = mouse::wants_pixels(mode).then(|| {
            let origin = self.layout.get().origin;
            let from_corner = |at: Pixels, corner: Pixels| f32::from(at - corner).max(0.0) as u32;
            (from_corner(position.x, origin.x), from_corner(position.y, origin.y))
        });
        if action == mouse::MouseAction::Motion && pixel.is_none() {
            if self.last_mouse_cell == Some((col, row)) {
                return true;
            }
            self.last_mouse_cell = Some((col, row));
        }
        let input = mouse::MouseInput {
            button,
            action,
            col,
            row,
            shift: modifiers.shift,
            alt: modifiers.alt,
            control: modifiers.control,
            pixel,
        };
        if let (Some(bytes), Some(pty)) = (mouse::encode(&input, mode), &self.pty) {
            pty.write(bytes);
            match (button, action) {
                (mouse::MouseButton::Left, mouse::MouseAction::Press) => self.app_selection.pressed(row),
                (mouse::MouseButton::Left, mouse::MouseAction::Release) => {
                    self.app_selection.released(row, Instant::now());
                }
                (mouse::MouseButton::Middle | mouse::MouseButton::Right, mouse::MouseAction::Press) => {
                    self.app_selection.forget();
                }
                _ => {}
            }
        }
        true
    }

    /// The pointer for a program that asked for the DEC locator: buttons and movement go to it as
    /// the locator's events, which it chose (`CSI Pm ' {`), not as mouse reports. Pixels are the
    /// window's own units, from the text area's corner.
    fn report_locator(
        &mut self,
        button: mouse::MouseButton,
        action: mouse::MouseAction,
        position: Point<Pixels>,
    ) -> bool {
        use mouse::MouseButton::{Left, Middle, Right};
        let Some((col, row)) = self.viewport_cell(position) else { return true };
        let layout = self.layout.get();
        let at = mouse::LocatorPointer {
            col,
            row,
            x: f32::from(position.x - layout.origin.x).max(0.0) as u32,
            y: f32::from(position.y - layout.origin.y).max(0.0) as u32,
        };
        let change = match (action, button) {
            (mouse::MouseAction::Motion, _) => None,
            (mouse::MouseAction::Press, Left | Middle | Right) => Some((button, true)),
            (mouse::MouseAction::Release, Left | Middle | Right) => Some((button, false)),
            // The wheel is not a locator button.
            _ => return true,
        };
        if let Some(pty) = &self.pty {
            pty.locator(change, at);
        }
        true
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if event.button == MouseButton::Left && event.modifiers.platform && self.open_link(event.position, cx)
        {
            return;
        }
        let button = match event.button {
            MouseButton::Left => mouse::MouseButton::Left,
            MouseButton::Middle => mouse::MouseButton::Middle,
            MouseButton::Right => mouse::MouseButton::Right,
            _ => return,
        };
        if self.report_mouse(button, mouse::MouseAction::Press, event.position, event.modifiers) {
            self.mouse_button = Some(button);
            return;
        }
        if event.button != MouseButton::Left {
            return;
        }
        let Some((point, side)) = self.grid_point(event.position) else { return };
        let kind = match event.click_count {
            2 => SelectionType::Semantic,
            3 => SelectionType::Lines,
            _ => SelectionType::Simple,
        };
        if let Some(pty) = &self.pty {
            pty.start_selection(kind, point, side);
        }
        self.selecting = true;
        cx.notify();
    }

    pub(super) fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        // A release we never saw (it happened somewhere we don't hear about)
        // shows as a move with no button down: end the drag now.
        if event.pressed_button.is_none() && (self.mouse_button.is_some() || self.selecting) {
            self.end_drag(event.position, event.modifiers, cx);
        }
        if !self.selecting {
            if event.pressed_button.is_none() {
                self.hover_link(event.position, event.modifiers.platform, cx);
                self.hover_block(event.position, cx);
            }
            let button = self.mouse_button.unwrap_or(mouse::MouseButton::None);
            self.report_mouse(button, mouse::MouseAction::Motion, event.position, event.modifiers);
            return;
        }
        // Off the grid (the pane's padding) the drag listener scrolls and follows.
        if event.pressed_button != Some(MouseButton::Left)
            || drag_scroll_lines(&self.layout.get(), event.position).is_some()
        {
            return;
        }
        let Some((point, side)) = self.grid_point(event.position) else { return };
        if let Some(pty) = &self.pty {
            pty.update_selection(point, side);
        }
        cx.notify();
    }

    /// The pointer moved outside the grid while a selection is dragged: extend
    /// the selection to the nearest edge cell, scrolling the history by the
    /// distance past the top or bottom.
    pub(super) fn drag_selection_outside(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !self.selecting || event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        let layout = self.layout.get();
        let Some(pty) = &self.pty else { return };
        let Some(lines) = drag_scroll_lines(&layout, event.position) else { return };
        if lines != 0 {
            pty.scroll(Scroll::Delta(lines));
        }
        let Some((col, row, side)) = cell_at(&layout, event.position, |row| self.row_size(row)) else {
            return;
        };
        let offset = pty.display_offset() as i32;
        pty.update_selection(GridPoint::new(Line(row as i32 - offset), Column(col)), side);
        cx.notify();
    }

    pub(super) fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.end_drag(event.position, event.modifiers, cx);
    }

    /// The button went up: report the release to an app tracking the mouse, or
    /// finish the local selection.
    fn end_drag(&mut self, position: Point<Pixels>, modifiers: Modifiers, cx: &mut Context<Self>) {
        if let Some(button) = self.mouse_button.take() {
            self.report_mouse(button, mouse::MouseAction::Release, position, modifiers);
            return;
        }
        self.selecting = false;
        if let Some(pty) = &self.pty {
            pty.finish_selection();
        }
        cx.notify();
    }

    pub(super) fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let mode = self.mode;
        if self.pty.is_none() {
            return;
        }
        if mouse::wants_mouse(mode) && !event.modifiers.shift {
            self.report_horizontal_scroll(event);
        }
        let cell_h = f32::from(self.layout.get().cell.height).max(1.0);
        let lines = match event.delta {
            ScrollDelta::Lines(delta) => delta.y,
            ScrollDelta::Pixels(delta) => f32::from(delta.y) / cell_h,
        } + self.scroll_remainder;
        let whole = lines.trunc();
        self.scroll_remainder = lines - whole;
        let whole = whole as i32;
        if whole == 0 {
            return;
        }

        if mouse::wants_mouse(mode) && !event.modifiers.shift {
            let button = if whole > 0 { mouse::MouseButton::WheelUp } else { mouse::MouseButton::WheelDown };
            for _ in 0..whole.unsigned_abs().min(10) {
                self.report_mouse(button, mouse::MouseAction::Press, event.position, event.modifiers);
            }
            return;
        }
        let Some(pty) = &self.pty else { return };
        if mode.contains(Modes::ALT_SCREEN) && mode.contains(Modes::ALTERNATE_SCROLL) {
            // Full-screen apps (vim, less) get arrow keys instead.
            let key: &[u8] = match (whole > 0, mode.contains(Modes::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            pty.write(key.repeat(whole.unsigned_abs() as usize));
        } else {
            pty.scroll(Scroll::Delta(whole));
            self.forget_block_cursor();
        }
        cx.notify();
    }

    /// Scroll the view to `offset` lines back from the live screen (the minimap's and the
    /// scrollbar's way of scrolling).
    pub(super) fn scroll_to_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        let lines = offset as i32 - pty.display_offset() as i32;
        if lines != 0 {
            pty.scroll(Scroll::Delta(lines));
            cx.notify();
        }
    }

    /// Sideways trackpad scrolling, reported as wheel-left/right to apps
    /// that asked for mouse events.
    pub(super) fn report_horizontal_scroll(&mut self, event: &ScrollWheelEvent) {
        let cell_w = f32::from(self.layout.get().cell.width).max(1.0);
        let cols = match event.delta {
            ScrollDelta::Lines(delta) => delta.x,
            ScrollDelta::Pixels(delta) => f32::from(delta.x) / cell_w,
        } + self.scroll_remainder_x;
        let whole = cols.trunc();
        self.scroll_remainder_x = cols - whole;
        let whole = whole as i32;
        let button = if whole > 0 { mouse::MouseButton::WheelLeft } else { mouse::MouseButton::WheelRight };
        for _ in 0..whole.unsigned_abs().min(10) {
            self.report_mouse(button, mouse::MouseAction::Press, event.position, event.modifiers);
        }
    }
}

/// The viewport cell under `position` (clamped to the grid) and which half of it. A row that is
/// double-width or half of a double-height line has cells twice as wide and only half of them, so
/// the column is the one the pointer is over in the row as it is drawn.
fn cell_at(
    layout: &Layout,
    position: Point<Pixels>,
    size_of: impl Fn(usize) -> LineSize,
) -> Option<(usize, usize, Side)> {
    if layout.cols == 0 || layout.rows == 0 {
        return None;
    }
    let x = f32::from(position.x - layout.origin.x).max(0.0);
    let y = f32::from(position.y - layout.origin.y).max(0.0);
    let cell_h = f32::from(layout.cell.height);
    let row = ((y / cell_h) as usize).min(layout.rows - 1);
    let size = size_of(row);
    let cell_w = f32::from(layout.cell.width) * super::line_size::width_scale(size);
    let col = ((x / cell_w) as usize).min(size.visible_columns(layout.cols) - 1);
    let side = if (x / cell_w).fract() < 0.5 { Side::Left } else { Side::Right };
    Some((col, row, side))
}

/// Lines to scroll (up is positive) for a drag at `position`, or `None` while
/// the pointer is over the grid, where the pane's own handler follows it.
fn drag_scroll_lines(layout: &Layout, position: Point<Pixels>) -> Option<i32> {
    if layout.rows == 0 {
        return None;
    }
    let top = layout.origin.y;
    let bottom = top + layout.cell.height * layout.rows as f32;
    let left = layout.origin.x;
    let right = left + layout.cell.width * layout.cols as f32;
    let cell_h = f32::from(layout.cell.height).max(1.0);
    if position.y < top {
        Some((f32::from(top - position.y) / cell_h).ceil() as i32)
    } else if position.y >= bottom {
        Some(-((f32::from(position.y - bottom) / cell_h).floor() as i32 + 1))
    } else if position.x < left || position.x >= right {
        Some(0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Keystroke, point, px, size};
    use wormhole_term::Direction;
    use wormhole_term::types::{LineSize, Side};

    use super::super::Layout;
    use super::{TerminalCommand, cell_at, drag_scroll_lines, keystroke_spec, platform_command};

    fn layout() -> Layout {
        Layout {
            origin: point(px(10.0), px(42.0)),
            cell: size(px(8.0), px(18.0)),
            cols: 80,
            rows: 24,
            gutter: 12.0,
            frame: (0.0, 0.0),
            spare: (0.0, 0.0),
        }
    }

    #[test]
    fn a_double_width_row_has_cells_twice_as_wide_and_only_half_of_them() {
        let layout = layout();
        let sizes = |row: usize| if row == 1 { LineSize::DoubleWidth } else { LineSize::Single };
        // x = 10 is the grid's left edge, a cell is 8 wide; row 1 is y 60..78.
        let at = |x: f32, y: f32| cell_at(&layout, point(px(x), px(y)), sizes);
        assert_eq!(at(10.0 + 8.0 * 5.0 + 1.0, 50.0), Some((5, 0, Side::Left)), "a normal row");
        assert_eq!(at(10.0 + 16.0 * 5.0 + 1.0, 65.0), Some((5, 1, Side::Left)), "five doubled cells in");
        assert_eq!(at(10.0 + 16.0 * 5.0 + 12.0, 65.0), Some((5, 1, Side::Right)), "the right half of it");
        assert_eq!(at(10.0 + 16.0 * 60.0, 65.0), Some((39, 1, Side::Left)), "past the half: the last shown");
        let tall = |_: usize| LineSize::DoubleHeightBottom;
        assert_eq!(cell_at(&layout, point(px(10.0 + 32.0), px(65.0)), tall), Some((2, 1, Side::Left)));
    }

    #[test]
    fn drags_past_the_edges_scroll_by_the_distance() {
        let layout = layout();
        assert_eq!(drag_scroll_lines(&layout, point(px(50.0), px(100.0))), None, "over the grid");
        assert_eq!(drag_scroll_lines(&layout, point(px(50.0), px(41.0))), Some(1));
        assert_eq!(drag_scroll_lines(&layout, point(px(50.0), px(42.0 - 37.0))), Some(3));
        let bottom = 42.0 + 18.0 * 24.0;
        assert_eq!(drag_scroll_lines(&layout, point(px(50.0), px(bottom))), Some(-1));
        assert_eq!(drag_scroll_lines(&layout, point(px(50.0), px(bottom + 20.0))), Some(-2));
        assert_eq!(drag_scroll_lines(&layout, point(px(2.0), px(100.0))), Some(0), "beside the grid");
    }

    #[test]
    fn terminal_shortcuts() {
        let command =
            |keys: &str| platform_command(&keystroke_spec(&Keystroke::parse(keys).expect("keystroke")));
        assert_eq!(command("cmd-c"), Some(TerminalCommand::Copy));
        assert_eq!(command("cmd-k"), Some(TerminalCommand::Clear));
        assert_eq!(command("cmd-backspace"), Some(TerminalCommand::Send(b"\x15")));
        assert_eq!(command("cmd-left"), Some(TerminalCommand::Send(b"\x01")));
        assert_eq!(command("cmd-up"), Some(TerminalCommand::Prompt(Direction::Up)), "jump between prompts");
        assert_eq!(command("cmd-down"), Some(TerminalCommand::Prompt(Direction::Down)));
        assert_eq!(command("shift-cmd-c"), Some(TerminalCommand::Copy), "extra modifiers, as before");
        assert_eq!(command("cmd-z"), None, "nothing to undo in a terminal");
        assert_eq!(command("cmd-t"), None, "⌘T belongs to the app");
        assert_eq!(command("ctrl-cmd-1"), None, "and so do the digits");
        assert_eq!(command("shift-cmd-e"), None, "an app key is never the terminal's");
    }

    /// F6 / ⇧F6 move the focus even from a terminal; the other F-keys and ⌘ keys of the
    /// terminal stay its own.
    #[test]
    fn the_terminal_gives_up_f6() {
        let yields = |keys: &str| {
            wormhole_core::keys::keymap::terminal_yields(&keystroke_spec(
                &Keystroke::parse(keys).expect("keystroke"),
            ))
        };
        assert!(yields("f6") && yields("shift-f6"));
        assert!(!yields("f5") && !yields("cmd-c") && !yields("tab") && !yields("escape"));
    }
}
