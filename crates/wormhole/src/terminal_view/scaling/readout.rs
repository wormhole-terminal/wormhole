//! The chip with the grid's size (core `view::motion::size_readout`, the component `style::size_readout`).

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::view::motion::size_readout::{Phase, phase};

use crate::settings::ui_in;
use crate::style;
use crate::terminal_view::TerminalView;

/// A text zoom older than this when a frame settles the grid says nothing: the terminal was not on screen.
const DUE_FOR: Duration = Duration::from_secs(2);

/// The chip, while it shows.
pub(super) struct Readout {
    text: SharedString,
    /// The last change of size.
    since: Instant,
    /// The draw that starts the fade (or removes the chip) when its time is up.
    wake: Option<Task<()>>,
}

impl Readout {
    /// What the chip says, for the tests.
    #[cfg(test)]
    pub(super) fn text(&self) -> String {
        self.text.to_string()
    }
}

impl TerminalView {
    /// A frame found the pane making a grid of `asked`: the chip says it when a live resize changed it, or
    /// when a text zoom has settled. The first size a terminal is seen at says nothing: never on open.
    pub(super) fn note_size(
        &mut self,
        asked: (usize, usize),
        live: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let changed = self.scaling.wanted.is_some_and(|before| before != asked);
        self.scaling.wanted = Some(asked);
        let now = cx.background_executor().now();
        let zoomed =
            self.scaling.readout_due.take().is_some_and(|at| now.saturating_duration_since(at) < DUE_FOR);
        if (live && changed) || zoomed {
            self.show_size(asked, now);
            // This frame's render is behind us, so the chip is drawn by the next.
            window.request_animation_frame();
        }
    }

    /// The chip says `cells` from `now` on; one that shows already says it afresh, its time starting again.
    fn show_size(&mut self, (cols, rows): (usize, usize), now: Instant) {
        let text = wormhole_text::keys::terminal_size_readout(&cols.to_string(), &rows.to_string());
        self.scaling.readout = Some(Readout { text: text.into(), since: now, wake: None });
    }

    /// The chip, in the middle of the terminal, if it shows: whole until its time is up, then fading.
    pub(in crate::terminal_view) fn render_size_readout(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let now = cx.background_executor().now();
        let readout = self.scaling.readout.as_mut()?;
        let opacity = match phase(now.saturating_duration_since(readout.since), cx.reduce_motion()) {
            Phase::Gone => {
                self.scaling.readout = None;
                return None;
            }
            Phase::Holding(left) => {
                if readout.wake.is_none() {
                    readout.wake = Some(cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(left).await;
                        this.update(cx, |_, cx| cx.notify()).ok();
                    }));
                }
                1.0
            }
            Phase::Fading(left) => {
                window.request_animation_frame();
                left
            }
        };
        let text = readout.text.clone();
        let ui = ui_in(&self.profile, cx);
        Some(
            div()
                .debug_selector(|| "size-readout".to_string())
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(style::size_readout(ui, text, opacity))
                .into_any_element(),
        )
    }
}
