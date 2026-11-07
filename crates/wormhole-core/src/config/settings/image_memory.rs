//! `image_memory_mib`: how much memory one terminal's inline images (kitty, iTerm2, Sixel) may take,
//! decoded. The range, the steps a settings page offers and the bytes the terminal layer is given
//! are decided here once, so the GPUI and the Swift settings show and apply the same thing.
//!
//! The limits themselves are the terminal layer's (`wormhole_term::images`: the hard ceiling and
//! the global budget derived from the per-terminal one); this only says them in MiB.

use std::ops::RangeInclusive;

use wormhole_term::images::{STORE_BUDGET, STORE_BUDGET_MAX, STORE_BUDGET_MIN, global_budget_for};

use super::Settings;

const MIB: usize = 1024 * 1024;

/// The MiB `image_memory_mib` is kept within.
pub const IMAGE_MEMORY_MIB: RangeInclusive<u32> =
    (STORE_BUDGET_MIN / MIB) as u32..=(STORE_BUDGET_MAX / MIB) as u32;

/// What a settings page offers; a value set in config.json shows as it is.
pub const IMAGE_MEMORY_STEPS_MIB: [u32; 5] = [64, 128, 256, 512, 1024];

/// The MiB a terminal gets when nothing is set.
pub const IMAGE_MEMORY_DEFAULT_MIB: u32 = (STORE_BUDGET / MIB) as u32;

impl Settings {
    /// The budget a terminal is started with, in bytes (`StartOptions::image_budget`).
    pub fn image_memory_bytes(&self) -> usize {
        self.image_memory_mib as usize * MIB
    }

    /// What all terminals' images may take together at this budget, in MiB: for the words under
    /// the setting, which say that the total is capped too.
    pub fn image_memory_all_mib(&self) -> u32 {
        (global_budget_for(self.image_memory_bytes()) / MIB) as u32
    }

    pub(super) fn sanitize_image_memory(&mut self) {
        self.image_memory_mib =
            self.image_memory_mib.clamp(*IMAGE_MEMORY_MIB.start(), *IMAGE_MEMORY_MIB.end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_are_in_range_and_include_the_default() {
        for step in IMAGE_MEMORY_STEPS_MIB {
            assert!(IMAGE_MEMORY_MIB.contains(&step), "{step}");
        }
        assert!(IMAGE_MEMORY_STEPS_MIB.contains(&IMAGE_MEMORY_DEFAULT_MIB));
        assert_eq!(IMAGE_MEMORY_STEPS_MIB.first(), Some(IMAGE_MEMORY_MIB.start()));
        assert_eq!(IMAGE_MEMORY_STEPS_MIB.last(), Some(IMAGE_MEMORY_MIB.end()));
        assert!(IMAGE_MEMORY_STEPS_MIB.is_sorted());
        assert_eq!(Settings::default().image_memory_mib, IMAGE_MEMORY_DEFAULT_MIB);
    }

    #[test]
    fn a_value_from_a_file_is_kept_within_the_range() {
        for (asked, kept) in [(0, 64), (63, 64), (100, 100), (1024, 1024), (1025, 1024), (u32::MAX, 1024)] {
            let mut settings = Settings { image_memory_mib: asked, ..Settings::default() };
            settings.sanitize_image_memory();
            assert_eq!(settings.image_memory_mib, kept, "{asked}");
        }
    }

    #[test]
    fn the_bytes_are_what_the_terminal_layer_keeps_the_budget_within() {
        for mib in IMAGE_MEMORY_STEPS_MIB {
            let settings = Settings { image_memory_mib: mib, ..Settings::default() };
            let bytes = settings.image_memory_bytes();
            assert_eq!(wormhole_term::images::clamp_budget(bytes), bytes, "{mib} MiB goes through unchanged");
        }
        let all = |mib| Settings { image_memory_mib: mib, ..Settings::default() }.image_memory_all_mib();
        assert_eq!((all(64), all(128), all(512), all(1024)), (256, 512, 2048, 2048));
    }
}
