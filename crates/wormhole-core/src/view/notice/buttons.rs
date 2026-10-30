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
    /// The capsule itself is the button: a click on it does what the words offer ("Restart"). It draws
    /// nothing of its own.
    pub act: bool,
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
    /// What the words offer happens (the app restarts into the update).
    Act,
    /// Nothing.
    Nothing,
}

impl Buttons {
    /// No buttons: the chip ignores the mouse.
    pub const NONE: Self = Self { info: false, close: false, act: false };

    /// The buttons of the chip that shows `notice`'s own words: only a notice that stays has any.
    #[must_use]
    pub fn of(notice: Notice) -> Self {
        Self { info: notice.has_info(), close: notice.is_sticky(), act: notice.has_action() }
    }

    /// How many there are (drawn ones: the capsule's own click takes no room).
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
            Press::Body if self.act => Reaction::Act,
            Press::Body if self.info => Reaction::ShowInfo,
            Press::Body if self.close => Reaction::Dismiss,
            Press::Close | Press::Info | Press::Body => Reaction::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::notice::{Copied, Failed, Refused, Warning};

    fn warned() -> Notice {
        Notice::Warned(Warning::HostFallback)
    }

    #[test]
    fn a_warning_that_has_more_to_say_gets_both_buttons() {
        assert_eq!(Buttons::of(warned()), Buttons { info: true, close: true, act: false });
        assert_eq!(Buttons::of(warned()).count(), 2);
    }

    #[test]
    fn a_warning_with_nothing_more_to_say_gets_only_the_cross() {
        let plain = Notice::Warned(Warning::Test(1));
        assert_eq!(Buttons::of(plain), Buttons { info: false, close: true, act: false });
    }

    #[test]
    fn a_copy_or_a_failure_has_no_buttons() {
        for notice in [
            Notice::from(Copied::Path),
            Notice::from(Copied::by_program("x")),
            Notice::from(Failed::CopyCommit),
            Notice::from(Failed::PutAtPrompt(Refused::Lines)),
            Notice::Test(1),
        ] {
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
    fn a_notice_that_offers_something_acts_on_the_body_and_closes_on_the_cross() {
        let offer = Buttons::of(Notice::Warned(Warning::UpdateReady(1)));
        assert_eq!(offer, Buttons { info: false, close: true, act: true });
        assert_eq!(offer.count(), 1, "the capsule is the button: only the cross is drawn");
        assert_eq!(offer.react(Press::Body), Reaction::Act);
        assert_eq!(offer.react(Press::Close), Reaction::Dismiss);
        assert_eq!(offer.react(Press::Info), Reaction::Nothing);
    }

    #[test]
    fn a_chip_without_buttons_reacts_to_nothing() {
        for press in [Press::Close, Press::Info, Press::Body] {
            assert_eq!(Buttons::NONE.react(press), Reaction::Nothing);
        }
    }
}
