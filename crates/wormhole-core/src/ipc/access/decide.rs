//! The one decision (`agent-permissions.md`, section 13): what a request is told, given the mode, what it
//! needs and what its program holds. Pure and small enough to be a truth table.

use super::caps::{CapSet, Capability, Need};
use crate::config::AgentAccess;
use crate::ipc::control::{Error, ErrorCode, Reason};

/// What a request that is let in may do beyond what needs no approval: the two switches the handlers
/// check, now per request, and which approval it came under.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Access {
    /// May read terminals other than its own (`control_read`'s old meaning).
    pub read: bool,
    /// May type into terminals and open them (`control_write`'s old meaning).
    pub write: bool,
    /// The approval it is under, so that taking it back ends the streams it started. `None` for a
    /// request that needed none, and for "Allowed while open".
    pub holder: Option<u64>,
}

impl Access {
    /// Everything: "Allowed while open".
    pub const OPEN: Access = Access { read: true, write: true, holder: None };
}

/// What a program holds, when something covers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowance {
    pub caps: CapSet,
    pub holder: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow(Access),
    /// The user decides: a question is shown, and the request waits for it or is refused with
    /// `approval_needed`.
    Ask,
    Refuse(Error),
}

/// Decide a request that `need`s something, in `mode`. `self_read`: it only reads the terminal it runs in,
/// which never needs approval. `held`: what covers the program; `cooling`: it was told no a moment ago.
pub fn decide(
    mode: AgentAccess,
    need: Need,
    self_read: bool,
    held: Option<Allowance>,
    cooling: bool,
) -> Verdict {
    let (Need::Capability(_) | Need::Approved) = need else { return Verdict::Allow(Access::default()) };
    if need == Need::Capability(Capability::Read) && self_read {
        return Verdict::Allow(Access::default());
    }
    match mode {
        AgentAccess::Off => Verdict::Refuse(
            Error::new(
                ErrorCode::NotAllowed,
                "agent access is off: turn it on in Settings → Coding agents → Agent access",
            )
            .because(Reason::AgentAccessOff),
        ),
        AgentAccess::Open => Verdict::Allow(Access::OPEN),
        AgentAccess::Ask => match held {
            Some(allowance) if covers(allowance.caps, need) => Verdict::Allow(Access {
                read: allowance.caps.contains(Capability::Read),
                write: need != Need::Capability(Capability::Read),
                holder: allowance.holder,
            }),
            _ if cooling => Verdict::Refuse(
                Error::new(
                    ErrorCode::NotAllowed,
                    "the user said no a moment ago; it is not asked again for two minutes: do not retry, ask the user",
                )
                .because(Reason::DeclinedRecently),
            ),
            _ => Verdict::Ask,
        },
    }
}

fn covers(caps: CapSet, need: Need) -> bool {
    match need {
        Need::Nothing | Need::Approved => true,
        Need::Capability(capability) => caps.contains(capability),
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, Allowance, Verdict, decide};
    use crate::config::AgentAccess::{Ask, Off, Open};
    use crate::ipc::access::caps::{CapSet, Capability, Need};
    use crate::ipc::control::Reason;

    const READ: Need = Need::Capability(Capability::Read);
    const TYPE: Need = Need::Capability(Capability::Type);

    fn held(caps: CapSet) -> Option<Allowance> {
        Some(Allowance { caps, holder: Some(7) })
    }

    fn reason(verdict: &Verdict) -> Option<Reason> {
        match verdict {
            Verdict::Refuse(error) => error.reason,
            _ => None,
        }
    }

    #[test]
    fn what_needs_nothing_is_let_through_in_every_mode() {
        for mode in [Off, Ask, Open] {
            assert_eq!(
                decide(mode, Need::Nothing, false, None, false),
                Verdict::Allow(Access::default()),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn reading_ones_own_terminal_never_asks() {
        for mode in [Off, Ask] {
            assert_eq!(decide(mode, READ, true, None, false), Verdict::Allow(Access::default()));
        }
        // Typing into the terminal it runs in is not that: the typing rules refuse the caller's own.
        assert_eq!(decide(Ask, TYPE, true, None, false), Verdict::Ask);
    }

    #[test]
    fn off_refuses_whatever_the_program_holds() {
        let verdict = decide(Off, TYPE, false, held(CapSet::STANDARD), false);
        assert_eq!(reason(&verdict), Some(Reason::AgentAccessOff));
    }

    #[test]
    fn open_lets_everybody_in_with_no_holder() {
        assert_eq!(decide(Open, TYPE, false, None, true), Verdict::Allow(Access::OPEN));
    }

    #[test]
    fn ask_asks_a_program_nothing_covers_and_not_one_that_was_just_told_no() {
        assert_eq!(decide(Ask, TYPE, false, None, false), Verdict::Ask);
        assert_eq!(reason(&decide(Ask, TYPE, false, None, true)), Some(Reason::DeclinedRecently));
    }

    #[test]
    fn a_program_that_holds_the_capability_is_let_in_under_its_approval() {
        let verdict = decide(Ask, TYPE, false, held(CapSet::STANDARD), false);
        assert_eq!(verdict, Verdict::Allow(Access { read: true, write: true, holder: Some(7) }));
        // Read only: the typing switch stays off.
        let verdict = decide(Ask, READ, false, held(CapSet::EMPTY.with(Capability::Read)), false);
        assert_eq!(verdict, Verdict::Allow(Access { read: true, write: false, holder: Some(7) }));
        // Typing without the read bit: output of a `run` stays hidden.
        let verdict = decide(Ask, TYPE, false, held(CapSet::EMPTY.with(Capability::Type)), false);
        assert_eq!(verdict, Verdict::Allow(Access { read: false, write: true, holder: Some(7) }));
    }

    #[test]
    fn a_program_that_lacks_the_capability_is_asked_again_to_widen_it() {
        assert_eq!(decide(Ask, TYPE, false, held(CapSet::EMPTY.with(Capability::Read)), false), Verdict::Ask);
        assert_eq!(
            reason(&decide(Ask, TYPE, false, held(CapSet::EMPTY.with(Capability::Read)), true)),
            Some(Reason::DeclinedRecently)
        );
    }

    #[test]
    fn close_needs_only_to_be_let_in() {
        let close = Need::Approved;
        assert_eq!(decide(Ask, close, false, None, false), Verdict::Ask);
        assert!(matches!(decide(Ask, close, false, held(CapSet::EMPTY), false), Verdict::Allow(_)));
    }
}
