//! Who a grant is for (`agent-permissions.md`, 4.3): the part of a program that does not change when it
//! is updated and cannot be chosen by whoever runs it.

use crate::agents::AgentId;
use crate::storage::store::AgentGrantKey;

/// How a program is told from another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Signed with a Developer ID: the signer (identifier and team) is who the user trusts, so every
    /// install of the program shares one approval and an update keeps it.
    DeveloperId,
    /// Apple's own software (`fm`): the identifier.
    Apple,
    /// Signed ad hoc or by a development certificate: an identifier proves nothing when anyone can pick
    /// it, so the code hash pins the program, and an update asks again.
    AdHoc,
    /// An agent that is a script run by an interpreter (`node …/cli.js`): the script's package folder.
    /// The interpreter's signature says nothing about which program runs on it.
    Script,
    /// No signature that can be read: the file itself, by path, size and time.
    Unsigned,
    /// Everything that is not a known agent: one identity for all of it.
    Scripts,
}

impl Kind {
    /// How the database spells it.
    pub fn word(self) -> &'static str {
        match self {
            Kind::DeveloperId => "developer_id",
            Kind::Apple => "apple",
            Kind::AdHoc => "ad_hoc",
            Kind::Script => "script",
            Kind::Unsigned => "unsigned",
            Kind::Scripts => "scripts",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        [Kind::DeveloperId, Kind::Apple, Kind::AdHoc, Kind::Script, Kind::Unsigned, Kind::Scripts]
            .into_iter()
            .find(|kind| kind.word() == word)
    }
}

/// A program as an approval knows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identity {
    pub kind: Kind,
    /// The registry agent it is, if it is one.
    pub agent: Option<AgentId>,
    /// The signing identifier of a Developer ID or Apple program; empty otherwise.
    pub identifier: String,
    pub team_id: String,
    /// What pins it when its signer does not: see [`Kind`].
    pub pin: String,
}

impl Identity {
    /// The one identity of everything that is not a known agent.
    pub fn scripts() -> Self {
        Self {
            kind: Kind::Scripts,
            agent: None,
            identifier: String::new(),
            team_id: String::new(),
            pin: String::new(),
        }
    }

    /// Whether the user may be offered "Always" for it: not for Scripts, which is anything (decided
    /// 2026-10-27: the most a Scripts question offers is "Until Wormhole Quits").
    pub fn can_be_remembered(&self) -> bool {
        self.kind != Kind::Scripts
    }

    /// The database's key for it.
    pub fn key(&self) -> AgentGrantKey {
        AgentGrantKey {
            kind: self.kind.word().to_string(),
            agent: self.agent.map(agent_word).unwrap_or_default(),
            identifier: self.identifier.clone(),
            team_id: self.team_id.clone(),
            pin: self.pin.clone(),
        }
    }

    /// The identity a stored key stands for; `None` for a kind or an agent this build does not know.
    pub fn from_key(key: &AgentGrantKey) -> Option<Self> {
        let agent = if key.agent.is_empty() { None } else { Some(agent_from_word(&key.agent)?) };
        Some(Self {
            kind: Kind::from_word(&key.kind)?,
            agent,
            identifier: key.identifier.clone(),
            team_id: key.team_id.clone(),
            pin: key.pin.clone(),
        })
    }
}

/// An agent's id as it is written in the database and in the registry's files (`claude`, `open-code`).
pub fn agent_word(id: AgentId) -> String {
    serde_json::to_value(id).ok().and_then(|value| value.as_str().map(str::to_string)).unwrap_or_default()
}

fn agent_from_word(word: &str) -> Option<AgentId> {
    serde_json::from_value(serde_json::Value::String(word.to_string())).ok()
}

#[cfg(test)]
mod tests {
    use super::{Identity, Kind, agent_word};
    use crate::agents::{AGENTS, AgentId};

    #[test]
    fn an_identity_round_trips_through_the_key_the_database_keeps() {
        let identity = Identity {
            kind: Kind::DeveloperId,
            agent: Some(AgentId::Claude),
            identifier: "com.anthropic.claude-code".into(),
            team_id: "Q6L2SF6YDW".into(),
            pin: String::new(),
        };
        assert_eq!(identity.key().agent, "claude");
        assert_eq!(Identity::from_key(&identity.key()), Some(identity));
        assert_eq!(Identity::from_key(&Identity::scripts().key()), Some(Identity::scripts()));
    }

    #[test]
    fn a_key_this_build_cannot_read_is_not_an_identity() {
        let mut key = Identity::scripts().key();
        key.kind = "teleport".into();
        assert_eq!(Identity::from_key(&key), None);
        let mut key = Identity::scripts().key();
        key.agent = "no-such-agent".into();
        assert_eq!(Identity::from_key(&key), None);
    }

    #[test]
    fn every_agent_has_a_word_that_reads_back_and_every_kind_too() {
        for agent in AGENTS {
            let word = agent_word(agent.id);
            assert!(!word.is_empty());
            let identity = Identity { agent: Some(agent.id), ..Identity::scripts() };
            assert_eq!(Identity::from_key(&identity.key()).and_then(|i| i.agent), Some(agent.id), "{word}");
        }
        for kind in [Kind::DeveloperId, Kind::Apple, Kind::AdHoc, Kind::Script, Kind::Unsigned, Kind::Scripts]
        {
            assert_eq!(Kind::from_word(kind.word()), Some(kind));
        }
    }

    #[test]
    fn only_scripts_cannot_be_remembered() {
        assert!(!Identity::scripts().can_be_remembered());
        assert!(Identity { kind: Kind::AdHoc, ..Identity::scripts() }.can_be_remembered());
    }
}
