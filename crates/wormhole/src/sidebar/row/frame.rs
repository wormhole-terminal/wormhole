//! What every row of one frame shares: the settings it is drawn by and how text is measured, for a
//! name that is cut off to fade out and a tooltip to know when it is needed.

use gpui_kit::*;

use super::super::input::SidebarInput;
use crate::settings::{UiColors, settings_in, ui_in};
use crate::style::{self, Metrics};

/// How far before its line's end a name too long for it fades out to nothing, in place of an
/// ellipsis: the name uses the whole width, and the × under the pointer lies over the faded end.
const NAME_FADE: f32 = 32.0;

/// How much room a name must have to spare to be drawn whole rather than faded.
const FIT_SLACK: f32 = 16.0;

/// What every row of one frame shares: the settings and how text is measured.
pub(in crate::sidebar) struct Frame {
    pub(in crate::sidebar) ui: UiColors,
    /// What rows, cards and heads measure at the person's density.
    pub(in crate::sidebar) metrics: Metrics,
    pub(in crate::sidebar) show_dot: bool,
    pub(in crate::sidebar) show_changes: bool,
    pub(in crate::sidebar) show_notes: bool,
    /// The ⌘N keycaps are showing.
    pub(in crate::sidebar) hints: bool,
    pub(in crate::sidebar) code_font: SharedString,
    pub(in crate::sidebar) width: f32,
    text_style: TextStyle,
    rem: Pixels,
}

impl Frame {
    pub(in crate::sidebar) fn new(
        input: &SidebarInput,
        width: f32,
        hints: bool,
        window: &mut Window,
        cx: &App,
    ) -> Self {
        let s = settings_in(&input.profile, cx);
        Self {
            ui: ui_in(&input.profile, cx),
            metrics: Metrics::of(s.density, s.show_groups_as_cards)
                .fading(s.fade_group_colors)
                .with_group_folders(s.show_group_folders),
            show_dot: s.sidebar_status_dot,
            show_changes: s.sidebar_changes,
            show_notes: s.sidebar_notes,
            hints,
            code_font: crate::fonts::code_font_in(&input.profile, cx).0,
            width,
            text_style: window.text_style(),
            rem: window.rem_size(),
        }
    }

    /// How wide `text` is in the interface font at `size` (rems) and `weight`.
    pub(in crate::sidebar) fn measure(
        &self,
        text: &str,
        size: f32,
        weight: FontWeight,
        window: &Window,
    ) -> f32 {
        if text.is_empty() || text.contains('\n') {
            return 0.0;
        }
        let mut run = self.text_style.to_run(text.len());
        run.font.weight = weight;
        let size = rems(size).to_pixels(self.rem);
        f32::from(
            window.text_system().shape_line(SharedString::from(text.to_string()), size, &[run], None).width(),
        )
    }

    /// For a line of text too long for `room`: the bytes that fade out over its last [`NAME_FADE`]
    /// points, and the room it is cut to there. `None` when it fits.
    pub(in crate::sidebar) fn fade(
        &self,
        text: &str,
        size: f32,
        weight: FontWeight,
        room: f32,
        window: &Window,
    ) -> Option<style::NameFade> {
        if text.is_empty() || text.contains('\n') || room <= NAME_FADE {
            return None;
        }
        let mut run = self.text_style.to_run(text.len());
        run.font.weight = weight;
        let size = rems(size).to_pixels(self.rem);
        let line = window.text_system().shape_line(SharedString::from(text.to_string()), size, &[run], None);
        // The room is worked out before layout, a few points off at times: a name that only just fits
        // fades too, so it never falls back to an ellipsis.
        if f32::from(line.width()) <= room - FIT_SLACK {
            return None;
        }
        let start = line.closest_index_for_x(px(room - NAME_FADE));
        let end = line.closest_index_for_x(px(room)).max(start);
        Some(style::NameFade { bytes: start..end, room })
    }
}
