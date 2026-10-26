//! Painting the terminal grid (and the blurred lines under the title bar).
//!
//! A frame takes one short lock to bring a `ScreenSnapshot` up to date (only
//! rows the engine reports changed are copied), then builds only rows whose
//! content or selection changed (`RowCache`; scrolled rows are found again by
//! content), shaping only words it hasn't seen before (`WordCache`), and paints
//! glyph by glyph from that cache (`FrameCache`). The view is
//! embedded cached (`window::cached`), so window frames the terminal didn't cause
//! (sidebar, tick, hover) reuse its last paint instead of running any of this.
//!
//! This file is the `Render` impl and what the canvas keeps between frames; the work is split
//! by what it is:
//!
//! - a frame: `prepare` (layout, snapshot, rows, the cursor and pictures), then `paint` (the
//!   layers in order); `frame`, `cache`, `rows`, `words` and `shaping` are what they build on
//! - the grid's parts: `glyphs` (box drawing), `underline`, `cursor`, `images`, `blink`, `blur`,
//!   `sized` (double-width and double-height rows)
//! - beside the grid: `overlays` (block marks, find ticks, the bell, the drag), `scrollbar`,
//!   `minimap`, `exited` (the bar of a finished shell)

mod blink;
mod blur;
mod cache;
mod cursor;
mod exited;
mod frame;
mod glyphs;
mod images;
mod minimap;
mod outline;
mod overlays;
mod paint;
mod pane_bar;
mod prepare;
mod rows;
mod scrollbar;
mod shaping;
mod sized;
mod underline;
mod words;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod guards;
#[cfg(test)]
mod word_tests;

use self::frame::FrameCache;
use self::overlays::bell_edge;
// Only devctl asks where the ship docks from outside the renderer.
#[cfg(feature = "devctl")]
pub(super) use self::prepare::dock_landing;
pub(super) use self::scrollbar::ScrollbarState;
use self::shaping::{ShapeKey, WordsKey};
use self::words::{Glyph, WordCache};
use super::blocks;
use super::input::TerminalCommand;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::theme::fonts::TERMINAL_FONT_SIZES as FONT_SIZES;
use wormhole_term::Pty;
use wormhole_term::types::MouseTracking;

use super::status::Status;
use super::{
    ClearBuffer, CopyCommand, CopyOutput, MIN_WIDTH, NextPrompt, PADDING_X, PADDING_Y, PreviousPrompt,
    RerunCommand, SelectOutput, TerminalView,
};
use crate::perf;
use crate::platform::scroller::ScrollerPrefs;
use crate::settings::{settings_in, store_in};
use crate::theme_bridge::token;
use gpui_kit::component::input as text_input;
use wormhole_term::ScreenSnapshot;
use wormhole_term::types::NamedColor;

/// Terminal grids painted since launch, for devctl's `paints` (frames actually drawn, which no
/// program in the terminal can see).
#[cfg(feature = "devctl")]
pub(crate) static PAINTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// What the canvas keeps between frames.
#[derive(Default)]
pub(super) struct RenderState {
    frame: FrameCache<ShapeKey, Vec<Glyph>, ShapedLine>,
    words: WordCache<WordsKey>,
    minimap: wormhole_term::minimap::MinimapSnapshot,
    /// Where the minimap was last drawn, for clicks on it.
    minimap_bounds: Bounds<Pixels>,
    /// Where the scrollbar was last drawn, if it was, for the pointer.
    scrollbar: Option<scrollbar::ScrollbarLayout>,
    /// The inline images as GPUI frames, one conversion per image.
    images: images::ImageCache,
    /// The faces learned and the stretched glyphs built for double-width rows.
    outlines: outline::Outlines,
}

impl RenderState {
    /// The last frame's copy of the screen.
    pub(super) fn snapshot(&self) -> &ScreenSnapshot {
        &self.frame.snapshot
    }

    /// How far the last frame was scrolled into the history.
    pub(super) fn display_offset(&self) -> usize {
        self.frame.snapshot.display_offset
    }

    /// How long the history was in the last frame.
    pub(super) fn history_size(&self) -> usize {
        self.frame.snapshot.history_size
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.redraw.rendered();
        let entity = cx.entity();
        let store = store_in(&self.profile, cx);
        let palette = store.palette.clone();
        // A zoom kept while Settings changed the size still stays within what Settings allows.
        let font_size_setting = (store.settings.font_size + self.shown_zoom(window, cx))
            .clamp(*FONT_SIZES.start(), *FONT_SIZES.end());
        let line_height_setting = store.settings.line_height;
        let ligatures = store.settings.font_ligatures;
        let layout_cell = self.layout.clone();
        let mut base_font = font(store.settings.font_family.clone());
        base_font.features = crate::fonts::ligature_features(ligatures);
        let focused = self.focus_handle.is_focused(window);
        let exited = self.status == Status::Exited;
        let draws_cursor = cursor::drawn(self.status) && !self.cursor_off;
        let reduce_motion = cx.reduce_motion();
        let bell_glow = self.bell_glow().then_some(store.ui.agent_attention);
        let ui = store.ui;
        let drop_tint = token(&store.tokens, "state.highlight");
        let find_tick = palette.find_current;
        let selecting = self.selecting;
        let prepaint_state = self.render_state.clone();
        let dock = self.dock;
        let minimap = store.settings.minimap;
        let paint_state = self.render_state.clone();
        let weak = entity.downgrade();
        let link_rows = self.hovered_link_rows(self.render_state.borrow().display_offset());
        let link_color = palette.foreground;
        let hovered_block = self.hovered_block();
        let background_override =
            self.render_state.borrow().snapshot().colors[NamedColor::Background].map(crate::palette::to_hsla);
        let has_blocks = self.pty.as_ref().is_some_and(Pty::has_blocks);

        let prepare = prepare::Prepare {
            font_size: font_size_setting,
            line_height: line_height_setting,
            ligatures,
            base_font,
            palette: palette.clone(),
            focused,
            draws_cursor,
            reduce_motion,
            live_resize: self.live_on(cx),
            dock,
            layout: layout_cell,
            state: prepaint_state,
            entity,
        };
        let paint = paint::Paint {
            state: paint_state,
            hovered_block,
            ui,
            focused,
            link_rows,
            link_color,
            find_tick,
            selecting,
            view: weak,
        };
        let grid = canvas(
            move |bounds, window, cx| {
                let span = perf::span();
                let frame = prepare.run(bounds, window, cx);
                span.end(perf::Part::Prepare);
                frame
            },
            move |bounds, frame, window, cx| {
                let Some(frame) = frame else { return };
                #[cfg(feature = "devctl")]
                PAINTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let span = perf::span();
                // A grid larger than its pane (held while the text zooms) stays inside the pane.
                match frame.clip {
                    Some(clip) => window.with_content_mask(Some(ContentMask { bounds: clip }), |window| {
                        paint.run(bounds, &frame, window, cx)
                    }),
                    None => paint.run(bounds, &frame, window, cx),
                }
                span.end(perf::Part::Paint);
            },
        )
        .size_full();

        div()
            .id("terminal")
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            // The Edit menu sends the text fields' own actions, so the terminal answers them too
            // (the keys come through `TerminalKeys` first); without a selection they do nothing.
            .on_action(cx.listener(|this, _: &text_input::Copy, _, cx| {
                this.run_command(TerminalCommand::Copy, cx);
            }))
            .on_action(cx.listener(|this, _: &text_input::Paste, _, cx| {
                this.run_command(TerminalCommand::Paste, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &text_input::SelectAll, _, cx| {
                this.run_command(TerminalCommand::SelectAll, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ClearBuffer, _, cx| {
                this.run_command(TerminalCommand::Clear, cx);
                cx.notify();
            }))
            // The command blocks' actions, only once the shell has shown it has integration:
            // without blocks their menu items are greyed out.
            .when(has_blocks, |this| {
                this.on_action(cx.listener(|this, _: &PreviousPrompt, _, cx| {
                    this.jump_prompt(wormhole_term::Direction::Up, cx)
                }))
                .on_action(cx.listener(|this, _: &NextPrompt, _, cx| {
                    this.jump_prompt(wormhole_term::Direction::Down, cx)
                }))
                .on_action(cx.listener(|this, _: &SelectOutput, _, cx| {
                    this.block_action(blocks::BlockAction::SelectOutput, cx)
                }))
                .on_action(cx.listener(|this, _: &CopyOutput, _, cx| {
                    this.block_action(blocks::BlockAction::CopyOutput, cx)
                }))
                .on_action(cx.listener(|this, _: &CopyCommand, _, cx| {
                    this.block_action(blocks::BlockAction::CopyCommand, cx)
                }))
                .on_action(cx.listener(|this, _: &RerunCommand, _, cx| {
                    this.block_action(blocks::BlockAction::PutAtPrompt, cx)
                }))
            })
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !hovered {
                    this.leave_blocks(cx);
                }
            }))
            .size_full()
            // A program that set the background (OSC 11) has it behind the padding too, as the
            // grid's cells have it; the theme's own is the window's and shows through.
            .when_some(background_override, |this, color| this.bg(color))
            .min_w(px(MIN_WIDTH))
            .pl(px(PADDING_X))
            .pr(px(Self::right_inset(&self.profile, cx)))
            .pb(px(PADDING_Y))
            .pt(px(PADDING_Y))
            // A program that takes the mouse gets clicks, not a text selection (⇧ still selects).
            .cursor(if self.hovered_link.is_some() {
                CursorStyle::PointingHand
            } else if self.mode.mouse_tracking() != MouseTracking::Off {
                CursorStyle::Arrow
            } else {
                CursorStyle::IBeam
            })
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, window, cx| {
                this.hover_link(window.mouse_position(), event.modifiers.platform, cx);
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            // A release over the title bar, the sidebar or outside the window
            // still ends the drag (and is reported to apps that track the mouse).
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            // A tint (not a border) so the grid keeps its size while files hover over it.
            .drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(drop_tint))
            .on_drop(cx.listener(Self::drop_files))
            .child(grid)
            .when(minimap, |this| this.child(self.render_minimap(cx)))
            // The minimap is the scroller when it is shown.
            .when(!minimap, |this| this.child(self.render_scrollbar(cx)))
            .children(self.render_block_readout(cx))
            .children(self.render_size_readout(window, cx))
            .children(bell_glow.map(|color| bell_edge(ui, color)))
            .when(exited, |this| this.child(self.render_exited(cx)))
            .children(self.render_pane_bar(exited, cx))
            .children(self.render_host_notice(cx))
    }
}

impl TerminalView {
    /// The size the view must have to hold the grid it has now (`panes::opening`: a terminal that must not be
    /// resized while its pane's box moves is laid out at this): whole cells and half a cell over, so no rounding
    /// takes a column or a row away, and the padding round them.
    pub fn size_for_grid(&self, cx: &App) -> (f32, f32) {
        let pane = self.pane_size();
        (
            (pane.cols as f32 + 0.5) * pane.cell.0 + PADDING_X + Self::right_inset(&self.profile, cx),
            (pane.rows as f32 + 0.5) * pane.cell.1 + 2.0 * PADDING_Y,
        )
    }

    /// How far the grid stays from the pane's right edge: the padding, and the minimap when it is
    /// shown or the scrollbar's gutter (the legacy style; the overlay one takes no room). The
    /// find bar lines up with it.
    pub fn right_inset(profile: &ProfileId, cx: &App) -> f32 {
        PADDING_X
            + if settings_in(profile, cx).minimap { minimap::MINIMAP_W } else { ScrollerPrefs::gutter(cx) }
    }
}
