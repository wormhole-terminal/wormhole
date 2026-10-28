//! What a click on the chip does. A copy's chip has no buttons and takes no click. A warning that stays has
//! a × to close it and, when there is more to say than one line holds, an ⓘ beside it that opens the
//! explanation; the rest of the capsule is a larger target for the same explanation, and closes the chip
//! only where there is none.

use super::message::Notice;

/// Which buttons the chip shows beside its words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    /// ⓘ: more about it.
    pub info: bool,
    /// ×: close it.
    pub close: bool,
}

/// Where on the chip a click was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Close,
    Info,
    /// Anywhere else on the capsule.
    Body,
}

/// What a click comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    /// The warning goes, for good.
    Dismiss,
    /// The explanation opens (or closes again).
    ShowInfo,
    /// Nothing.
    Nothing,
}

impl Buttons {
    /// No buttons: the chip ignores the mouse.
    pub const NONE: Self = Self { info: false, close: false };

    /// The buttons of the chip that shows `notice`'s own words: only a notice that stays has any.
    #[must_use]
    pub fn of(notice: Notice) -> Self {
        Self { info: notice.has_info(), close: notice.is_sticky() }
    }

    /// How many there are.
    #[must_use]
    pub fn count(self) -> usize {
        usize::from(self.info) + usize::from(self.close)
    }

    /// What a click at `press` comes to. A button that is not there is not clicked, and a click on the body
    /// takes what the chip has: the explanation, which is the larger target for the ⓘ, or else it closes.
    #[must_use]
    pub fn react(self, press: Press) -> Reaction {
        match press {
            Press::Close if self.close => Reaction::Dismiss,
            Press::Info if self.info => Reaction::ShowInfo,
            Press::Body if self.info => Reaction::ShowInfo,
            Press::Body if self.close => Reaction::Dismiss,
            Press::Close | Press::Info | Press::Body => Reaction::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::notice::{Copied, Failed, Warning};

    fn warned() -> Notice {
        Notice::Warned(Warning::HostFallback)
    }

    #[test]
    fn a_warning_that_has_more_to_say_gets_both_buttons() {
        assert_eq!(Buttons::of(warned()), Buttons { info: true, close: true });
        assert_eq!(Buttons::of(warned()).count(), 2);
    }

    #[test]
    fn a_warning_with_nothing_more_to_say_gets_only_the_cross() {
        let plain = Notice::Warned(Warning::Test(1));
        assert_eq!(Buttons::of(plain), Buttons { info: false, close: true });
    }

    #[test]
    fn a_copy_or_a_failure_has_no_buttons() {
        for notice in [Notice::from(Copied::Path), Notice::from(Failed::CopyCommit), Notice::Test(1)] {
            assert_eq!(Buttons::of(notice), Buttons::NONE);
            assert_eq!(Buttons::of(notice).count(), 0);
        }
    }

    #[test]
    fn the_cross_closes_and_the_info_and_the_body_open_the_explanation() {
        let both = Buttons::of(warned());
        assert_eq!(both.react(Press::Close), Reaction::Dismiss);
        assert_eq!(both.react(Press::Info), Reaction::ShowInfo);
        assert_eq!(both.react(Press::Body), Reaction::ShowInfo, "the body no longer closes it");
    }

    #[test]
    fn the_body_closes_a_warning_that_has_no_explanation() {
        let cross = Buttons::of(Notice::Warned(Warning::Test(1)));
        assert_eq!(cross.react(Press::Body), Reaction::Dismiss);
        assert_eq!(cross.react(Press::Close), Reaction::Dismiss);
        assert_eq!(cross.react(Press::Info), Reaction::Nothing, "there is no such button");
    }

    #[test]
    fn a_chip_without_buttons_reacts_to_nothing() {
        for press in [Press::Close, Press::Info, Press::Body] {
            assert_eq!(Buttons::NONE.react(press), Reaction::Nothing);
        }
    }
}
