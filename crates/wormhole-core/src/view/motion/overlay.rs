//! The command palette coming and going, which the design fixed (the picks of the motion plan: no choice of
//! variant, so no `MotionChoices` field): it appears at once, as the keys are pressed, and closing fades
//! it, scrim and card together, in 100 ms. Only the exit has numbers. While it fades the palette is already
//! closed as far as the window is concerned: the keyboard is back where it was and the fading picture takes
//! no input.

use std::time::Duration;

use super::ease::EASE_IN;

/// The palette fading out, scrim and card together.
pub const EXIT: Duration = Duration::from_millis(100);

/// How much of the palette shows through `t` (0.0..=1.0) of the way through [`EXIT`]: all of it at the
/// start and none at the end, easing in as a thing that leaves does.
#[must_use]
pub fn exit_opacity(t: f32) -> f32 {
    1.0 - EASE_IN.ease(t.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exit_is_a_tenth_of_a_second_the_design_picked() {
        assert_eq!(EXIT, Duration::from_millis(100));
    }

    #[test]
    fn the_palette_starts_whole_ends_gone_and_only_ever_fades() {
        assert_eq!(exit_opacity(0.0), 1.0);
        assert_eq!(exit_opacity(1.0), 0.0);
        let mut before = 1.0;
        for step in 1..=20 {
            let now = exit_opacity(step as f32 / 20.0);
            assert!(now <= before, "{now} after {before}");
            before = now;
        }
        assert_eq!((exit_opacity(-1.0), exit_opacity(9.0)), (1.0, 0.0), "outside its time it is clamped");
    }

    #[test]
    fn it_eases_in_so_it_holds_on_a_little_before_it_goes() {
        assert!(exit_opacity(0.5) > 0.5, "{}", exit_opacity(0.5));
    }
}
