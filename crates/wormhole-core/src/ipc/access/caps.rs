//! What a grant lets a program do (`agent-permissions.md`, section 5), and which one each request needs.

use crate::ipc::control::{Action, Topic};

/// One kind of thing a program may be allowed to do to Wormhole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Read what terminals show: `read`, `blocks`, the output of `events`, `wait --match`.
    Read,
    /// Type into terminals: `send-text`, `send-keys`, `mouse`.
    Type,
    /// Have Wormhole type and run a command: `run`, and a command given to a new terminal.
    Run,
    /// Open terminals, windows and splits, and name and focus them.
    Open,
    /// Close a terminal. Every close is still the user's to answer, whatever the grant says.
    Close,
}

impl Capability {
    pub const ALL: [Capability; 5] =
        [Capability::Read, Capability::Type, Capability::Run, Capability::Open, Capability::Close];

    const fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// A set of [`Capability`], stored as the bits the database keeps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapSet(u32);

impl CapSet {
    pub const EMPTY: CapSet = CapSet(0);

    /// What the connection sheet grants: everything but `close`.
    pub const STANDARD: CapSet = CapSet(
        Capability::Read.bit() | Capability::Type.bit() | Capability::Run.bit() | Capability::Open.bit(),
    );

    /// The set stored as `bits`; bits no capability has are dropped.
    pub fn from_bits(bits: u32) -> Self {
        Self(bits & Capability::ALL.iter().fold(0, |all, cap| all | cap.bit()))
    }

    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn contains(self, capability: Capability) -> bool {
        self.0 & capability.bit() != 0
    }

    #[must_use]
    pub fn with(self, capability: Capability) -> Self {
        Self(self.0 | capability.bit())
    }

    #[must_use]
    pub fn without(self, capability: Capability) -> Self {
        Self(self.0 & !capability.bit())
    }
}

/// What a request needs from the program that makes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// Nothing: it shows titles, folders and states, never text (`list`, `status`, `notify`, `doctor`,
    /// `events` without output, a `wait` on states), or it has a key of its own (`welcome`).
    Nothing,
    /// A capability.
    Capability(Capability),
    /// To be let in at all, nothing more: `close` asks the user on every call anyway.
    Approved,
}

impl Need {
    /// What `action` needs. A match without a catch-all: an action added without saying what it needs
    /// does not compile.
    pub fn of(action: &Action) -> Need {
        use Capability::{Open, Read, Run, Type};
        match action {
            Action::List | Action::Doctor | Action::Status { .. } | Action::Notify { .. } => Need::Nothing,
            Action::Welcome { .. } => Need::Nothing,
            Action::Events(spec) => {
                // A spec that cannot be read is answered `bad_request` by the handler, asking for nothing.
                if spec.filter().is_ok_and(|filter| filter.topics.contains(Topic::Output)) {
                    Need::Capability(Read)
                } else {
                    Need::Nothing
                }
            }
            Action::Wait(spec) => {
                if spec.plan().is_ok_and(|plan| plan.needs_read()) {
                    Need::Capability(Read)
                } else {
                    Need::Nothing
                }
            }
            Action::Read(_) | Action::Blocks(_) => Need::Capability(Read),
            Action::SendText { .. } | Action::SendKeys(_) | Action::Mouse(_) => Need::Capability(Type),
            Action::Run(_) => Need::Capability(Run),
            Action::NewTerminal { command, .. } => {
                Need::Capability(if command.is_some() { Run } else { Open })
            }
            Action::NewWindow(spec) => Need::Capability(if spec.command.is_some() { Run } else { Open }),
            Action::Split(spec) => Need::Capability(if spec.command.is_some() { Run } else { Open }),
            Action::Focus(_) | Action::Rename(_) => Need::Capability(Open),
            Action::Close(_) => Need::Approved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CapSet, Capability, Need};
    use crate::ipc::control::{Action, Target};

    #[test]
    fn a_set_holds_what_was_put_in_and_drops_unknown_bits() {
        let set = CapSet::EMPTY.with(Capability::Read).with(Capability::Open);
        assert!(set.contains(Capability::Read) && set.contains(Capability::Open));
        assert!(!set.contains(Capability::Type) && !set.contains(Capability::Close));
        assert!(!set.without(Capability::Read).contains(Capability::Read));
        assert_eq!(CapSet::from_bits(set.bits() | 1 << 20), set, "a bit no capability has is dropped");
        assert!(!CapSet::STANDARD.contains(Capability::Close), "close is never part of the first grant");
        for cap in [Capability::Read, Capability::Type, Capability::Run, Capability::Open] {
            assert!(CapSet::STANDARD.contains(cap), "{cap:?}");
        }
    }

    #[test]
    fn what_each_kind_of_request_needs() {
        let terminal = Target::Terminal(3);
        let need = |json: serde_json::Value| {
            let request: crate::ipc::control::Request =
                serde_json::from_value(json.as_object().map(|_| json.clone()).expect("an object"))
                    .expect("parses");
            Need::of(&request.action)
        };
        let v = |action: &str, rest: serde_json::Value| {
            let mut object = serde_json::json!({ "version": 1, "action": action });
            if let (Some(base), Some(extra)) = (object.as_object_mut(), rest.as_object()) {
                base.extend(extra.clone());
            }
            object
        };
        let t = serde_json::json!({ "target": { "kind": "terminal", "id": terminal.id() } });
        for open in ["list", "doctor", "status", "events"] {
            assert_eq!(need(v(open, serde_json::json!({}))), Need::Nothing, "{open}");
        }
        assert_eq!(
            need(v("events", serde_json::json!({ "topics": ["output"] }))),
            Need::Capability(Capability::Read)
        );
        assert_eq!(need(v("read", t.clone())), Need::Capability(Capability::Read));
        assert_eq!(need(v("blocks", t.clone())), Need::Capability(Capability::Read));
        assert_eq!(
            need(v("send_text", serde_json::json!({ "target": t["target"], "text": "x" }))),
            Need::Capability(Capability::Type)
        );
        assert_eq!(need(v("new_terminal", serde_json::json!({}))), Need::Capability(Capability::Open));
        assert_eq!(
            need(v("new_terminal", serde_json::json!({ "command": "make" }))),
            Need::Capability(Capability::Run)
        );
        assert_eq!(need(v("new_window", serde_json::json!({}))), Need::Capability(Capability::Open));
        assert_eq!(
            need(v("new_window", serde_json::json!({ "command": "ls" }))),
            Need::Capability(Capability::Run)
        );
        assert_eq!(need(v("close", t)), Need::Approved);
        assert_eq!(Need::of(&Action::Doctor), Need::Nothing);
    }
}
