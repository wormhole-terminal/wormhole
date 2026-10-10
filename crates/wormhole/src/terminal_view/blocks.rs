//! Command blocks in the pane: the gutter mark beside each command, a hovered finished
//! block (a tint over its rows and a readout at the pane's bottom right with its exit code, its
//! time and Copy output), ⌘↑ / ⌘↓ between prompts, and the Edit menu's actions on the current
//! block. The blocks themselves are core's (`terminal::blocks`), read from the grid; the view
//! keeps only which one is hovered and where the last jump landed.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::alacritty_terminal::grid::Scroll;
use wormhole_core::alacritty_terminal::index::Line;
use wormhole_core::spoken;
use wormhole_core::terminal::{Block, BlockState, Direction, took_text};

use super::{Layout, PADDING_X, TerminalView};
use crate::settings::{SettingsStore, UiColors};
use crate::style;

/// The gutter mark: a 2 pt bar inside the pane's left padding, 1 pt from its edge, inset where
/// the block starts or ends in view (the grid's inset here is [`PADDING_X`]).
pub(super) const MARK_W: f32 = 2.0;
pub(super) const MARK_X: f32 = 1.0;
pub(super) const MARK_INSET: f32 = 2.0;

/// What the view keeps about blocks.
#[derive(Default)]
pub(super) struct Blocks {
    /// A finished block under the pointer: tinted, with the readout.
    hovered: Option<u64>,
    /// The pointer is on the readout itself, which stays while it is.
    readout_hovered: bool,
    /// The block ⌘↑ / ⌘↓ last landed on: where the next jump starts and what the Edit menu's
    /// actions act on, until the view is scrolled or typed in.
    cursor: Option<u64>,
}

/// The Edit menu's actions on a block (and devctl's `block`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BlockAction {
    SelectOutput,
    CopyOutput,
    CopyCommand,
    Rerun,
}

/// The gutter mark's color for a block in this state: the status colors for a command that
/// finished, faint for one running or a prompt nothing has run at, none where nothing ran.
pub(super) fn mark_color(state: BlockState, ui: UiColors) -> Option<Hsla> {
    match state {
        BlockState::Prompt | BlockState::Running { .. } => Some(ui.faint),
        BlockState::Finished { code: Some(0), .. } => Some(ui.success),
        BlockState::Finished { code: Some(_), .. } => Some(ui.danger),
        BlockState::Finished { code: None, took: Some(_) } => Some(ui.faint),
        BlockState::Finished { code: None, took: None } => None,
    }
}

impl TerminalView {
    /// The pointer moved over the pane: the finished block under it gets the tint and the readout.
    pub(super) fn hover_block(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if self.blocks.readout_hovered {
            return;
        }
        let hovered = block_row(&self.layout.get(), position).and_then(|row| {
            let state = self.render_state.borrow();
            let span = state.snapshot().block_at_row(row)?;
            (matches!(span.state, BlockState::Finished { .. }) && span.state.ran()).then_some(span.id)
        });
        self.set_hovered_block(hovered, cx);
    }

    /// The pointer left the pane.
    pub(super) fn leave_blocks(&mut self, cx: &mut Context<Self>) {
        if !self.blocks.readout_hovered {
            self.set_hovered_block(None, cx);
        }
    }

    fn set_hovered_block(&mut self, hovered: Option<u64>, cx: &mut Context<Self>) {
        if hovered != self.blocks.hovered {
            self.blocks.hovered = hovered;
            cx.notify();
        }
    }

    /// The hovered block, for the paint (its rows are the snapshot's).
    pub(super) fn hovered_block(&self) -> Option<u64> {
        self.blocks.hovered
    }

    /// The view scrolled or was typed in: the next jump starts from what is in view.
    pub(super) fn forget_block_cursor(&mut self) {
        self.blocks.cursor = None;
    }

    /// ⌘↑ / ⌘↓: the previous or next prompt comes to the top of the view, as Terminal.app's marks
    /// do. From the live screen, up goes to the prompt before the current one; past the last
    /// prompt, down returns to the live screen. VoiceOver hears the block landed on.
    pub(super) fn jump_prompt(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        let at_bottom = pty.display_offset() == 0;
        // From the block the last jump landed on (a prompt near the bottom may not have been
        // able to scroll to the top, so the view alone doesn't say where that was), else the
        // live prompt, else the top of the view (strictly: a prompt on the top row is passed).
        let from = match self.blocks.cursor.and_then(|id| pty.block(id)) {
            Some(block) => Some(block.start),
            None if at_bottom => pty.prompt_line(Line(pty.bottommost_line().0 + 1), Direction::Up),
            None => Some(Line(-(pty.display_offset() as i32))),
        };
        let Some(from) = from else { return };
        match pty.prompt_line(from, direction) {
            Some(line) => {
                pty.scroll_line_to_top(line);
                let block = pty.block_at(line);
                self.blocks.cursor = block.as_ref().map(|block| block.id);
                if let Some(block) = block {
                    crate::a11y::announce(&spoken::command_block(&block.command, block.state), false);
                }
            }
            None if direction == Direction::Down => {
                if !at_bottom {
                    pty.scroll(Scroll::Bottom);
                }
                self.blocks.cursor = None;
            }
            None => {}
        }
        cx.notify();
    }

    /// The block the Edit menu's actions act on: the one the last jump landed on (until the view
    /// is scrolled or typed in), else the newest one whose command ran.
    pub(super) fn target_block(&self) -> Option<Block> {
        let pty = self.pty.as_ref()?;
        if let Some(block) = self.blocks.cursor.and_then(|id| pty.block(id)) {
            return Some(block);
        }
        pty.blocks().into_iter().rev().find(|block| block.state.ran())
    }

    /// One of the Edit menu's block actions on the target block.
    pub(super) fn block_action(&mut self, action: BlockAction, cx: &mut Context<Self>) {
        if let Some(block) = self.target_block() {
            self.run_block_action(&block, action, cx);
        }
    }

    /// The readout's button: Copy Output for the hovered block.
    pub(super) fn copy_block_output(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(block) = self.pty.as_ref().and_then(|pty| pty.block(id)) {
            self.run_block_action(&block, BlockAction::CopyOutput, cx);
        }
    }

    /// Select Output selects the block's output lines and shows them; Copy Output and Copy
    /// Command put them on the clipboard; Rerun Command types the command at the prompt (the
    /// line the shell has so far cleared with ⌃U, as ⌘⌫ does) and runs it, only while the shell
    /// is at its prompt, since a running program would get the text as input.
    pub(super) fn run_block_action(&mut self, block: &Block, action: BlockAction, cx: &mut Context<Self>) {
        let Some(pty) = &self.pty else { return };
        match action {
            BlockAction::SelectOutput => {
                let Some((first, last)) = block.output_lines() else { return };
                pty.select_lines(first, last);
                pty.reveal(first);
                cx.notify();
            }
            BlockAction::CopyOutput => {
                let Some((first, last)) = block.output_lines() else { return };
                cx.write_to_clipboard(ClipboardItem::new_string(pty.lines_text(first, last)));
            }
            BlockAction::CopyCommand => {
                if !block.command.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(block.command.clone()));
                }
            }
            BlockAction::Rerun => {
                if block.command.is_empty() {
                    return;
                }
                if pty.at_prompt() != Some(true) {
                    tracing::info!("rerun: the shell isn't at its prompt");
                    return;
                }
                self.write_input(b"\x15".to_vec());
                self.paste(&block.command);
                self.write_input(b"\r".to_vec());
                cx.notify();
            }
        }
    }

    /// The readout for the hovered block: its exit code and time, and Copy output. At the
    /// pane's bottom right, over the grid; it stays while the pointer is on it.
    pub(super) fn render_block_readout(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = self.blocks.hovered?;
        let state = self.render_state.borrow();
        let span = state.snapshot().block_spans().iter().find(|span| span.id == id)?;
        let BlockState::Finished { code, took } = span.state else { return None };
        let ui = cx.global::<SettingsStore>().ui;
        let right = px(Self::right_inset(cx));
        let readout = style::block_readout(ui, code, took.map(took_text))
            .id("block-readout")
            .absolute()
            .bottom_2()
            .right(right)
            .mr_2()
            .cursor(CursorStyle::Arrow)
            // The pane below it would start a selection.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.blocks.readout_hovered = *hovered;
                if !hovered {
                    this.set_hovered_block(None, cx);
                }
            }))
            .child(
                style::icon_button("block-copy-output", ui, IconName::Copy, "Copy output")
                    .in_row()
                    .on_click(cx.listener(move |this, _, _, cx| this.copy_block_output(id, cx))),
            );
        Some(readout.into_any_element())
    }
}

/// devctl's view of the blocks and the inline images.
#[cfg(feature = "devctl")]
impl TerminalView {
    /// `images`: one line per image in the store: its id, size, bytes and placements; the store's
    /// total first.
    pub fn devctl_images(&self) -> String {
        let Some(pty) = &self.pty else { return String::new() };
        let mut store = pty.images();
        let mut out = format!("{} images, {} bytes\n", store.len(), store.bytes());
        let mut ids: Vec<u32> = store.ids().collect();
        ids.sort_unstable();
        for id in ids {
            let (width, height) = store.dimensions(id).unwrap_or_default();
            let decoded =
                store.get(id).map_or("decoding".to_string(), |image| format!("{} bytes", image.bytes()));
            let placements: Vec<String> = store
                .placements_of(id)
                .map(|(p, spec)| {
                    let shown = if store.is_shown(id, p) { "" } else { " hidden" };
                    format!(
                        "p{p}: {}x{} cells{}{shown}",
                        spec.cols,
                        spec.rows,
                        if spec.fit { " fit" } else { "" }
                    )
                })
                .collect();
            out.push_str(&format!("{id}: {width}x{height} {decoded} | {}\n", placements.join(", ")));
        }
        out
    }

    /// `blocks`: one line per block, oldest first: its number, its lines, how its command went
    /// and the command.
    pub fn devctl_blocks(&self) -> String {
        let Some(pty) = &self.pty else { return String::new() };
        let mut out = String::new();
        for (n, block) in pty.blocks().iter().enumerate() {
            let state = match block.state {
                BlockState::Prompt => "prompt".to_string(),
                BlockState::Running { .. } => "running".to_string(),
                BlockState::Finished { .. } => {
                    wormhole_core::terminal::readout_text(block.state).unwrap_or_else(|| "nothing ran".into())
                }
            };
            let output =
                block.output_lines().map(|(a, b)| format!(" output {}..={}", a.0, b.0)).unwrap_or_default();
            out.push_str(&format!(
                "{}: lines {}..={}{output} | {state} | {}\n",
                n + 1,
                block.start.0,
                block.end.0,
                block.command.replace('\n', "⏎")
            ));
        }
        out
    }

    /// `block <n> <select|copy-output|copy-command|rerun>`.
    pub fn devctl_block(&mut self, n: usize, action: &str, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let action = match action {
            "select" => BlockAction::SelectOutput,
            "copy-output" => BlockAction::CopyOutput,
            "copy-command" => BlockAction::CopyCommand,
            "rerun" => BlockAction::Rerun,
            other => anyhow::bail!("expected select, copy-output, copy-command or rerun, got {other:?}"),
        };
        let block = self
            .pty
            .as_ref()
            .and_then(|pty| pty.blocks().into_iter().nth(n))
            .ok_or_else(|| anyhow::anyhow!("no block {}", n + 1))?;
        self.run_block_action(&block, action, cx);
        Ok(())
    }
}

/// The viewport row under `position`: anywhere across the pane (the gutter included) within the
/// grid's rows.
fn block_row(layout: &Layout, position: Point<Pixels>) -> Option<usize> {
    if layout.rows == 0 {
        return None;
    }
    let y = f32::from(position.y - layout.origin.y);
    let cell_h = f32::from(layout.cell.height).max(1.0);
    let x = f32::from(position.x - layout.origin.x);
    (y >= 0.0 && y < cell_h * layout.rows as f32 && x >= -PADDING_X).then(|| (y / cell_h) as usize)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use gpui_kit::{point, px, size};
    use wormhole_core::terminal::BlockState;

    use super::super::Layout;
    use super::{block_row, mark_color};
    use crate::settings::UiColors;

    #[test]
    fn the_gutter_mark_takes_the_status_color_and_nothing_where_nothing_ran() {
        let tokens = wormhole_core::tokens::builtin_tokens();
        let ui = UiColors::new(&tokens[0], wormhole_core::tokens::Translucency { opacity: 1.0, blur: false });
        let finished = |code, took| BlockState::Finished { code, took };
        assert_eq!(mark_color(finished(Some(0), Some(Duration::from_secs(1))), ui), Some(ui.success));
        assert_eq!(mark_color(finished(Some(2), None), ui), Some(ui.danger));
        assert_eq!(mark_color(finished(None, Some(Duration::from_secs(1))), ui), Some(ui.faint));
        assert_eq!(mark_color(finished(None, None), ui), None, "an empty line");
        assert_eq!(mark_color(BlockState::Running { since: Instant::now() }, ui), Some(ui.faint));
        assert_eq!(mark_color(BlockState::Prompt, ui), Some(ui.faint));
    }

    #[test]
    fn a_hover_counts_across_the_pane_within_the_grids_rows() {
        let layout =
            Layout { origin: point(px(10.0), px(42.0)), cell: size(px(8.0), px(18.0)), cols: 80, rows: 24 };
        assert_eq!(block_row(&layout, point(px(50.0), px(42.0))), Some(0));
        assert_eq!(block_row(&layout, point(px(50.0), px(42.0 + 18.0 * 3.0 + 1.0))), Some(3));
        assert_eq!(block_row(&layout, point(px(7.0), px(100.0))), Some(3), "the gutter counts");
        assert_eq!(block_row(&layout, point(px(50.0), px(41.0))), None, "above the grid");
        assert_eq!(block_row(&layout, point(px(50.0), px(42.0 + 18.0 * 24.0))), None, "below it");
    }
}
