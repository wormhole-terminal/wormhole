//! The programs the user let control Wormhole for good ("Always" on the connection sheet): one row
//! per identity, revocable one by one (`agent-permissions.md`, section 9). The store holds rows and
//! their key as plain text; what an identity is, and which of its parts are the key, is the access
//! part of `ipc`'s (this module is below it and names none of its types).

use anyhow::Result;
use rusqlite::{OptionalExtension, params};

use super::{Store, exec};

/// What makes a row the same program again. Empty text stands for a part the kind does not have.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentGrantKey {
    /// `developer_id`, `ad_hoc`, `script`, `unsigned` or `scripts`.
    pub kind: String,
    /// The agent's registry id (`claude`), for the kinds that are an agent.
    pub agent: String,
    /// The signing identifier of a Developer ID program.
    pub identifier: String,
    pub team_id: String,
    /// What pins the program when its signer does not: an ad-hoc program's hash, a script's package
    /// folder, an unsigned program's path with its size and time.
    pub pin: String,
}

/// A stored approval.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentGrantRow {
    pub id: i64,
    pub key: AgentGrantKey,
    /// The program's name when it was approved ("Claude Code"), for the list in Settings.
    pub label: String,
    /// Where it ran from, last time (display only, not part of the key).
    pub location: String,
    /// Its code hash, last time (display only, unless the kind pins by it).
    pub cdhash: String,
    /// The capabilities granted, as the bits `ipc::access::CapSet` defines.
    pub caps: u32,
    /// Unix seconds.
    pub created: i64,
    pub last_used: Option<i64>,
    pub uses: i64,
}

const COLUMNS: &str =
    "id, kind, agent, identifier, team_id, pin, label, location, cdhash, caps, created, last_used, uses";

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentGrantRow> {
    Ok(AgentGrantRow {
        id: row.get(0)?,
        key: AgentGrantKey {
            kind: row.get(1)?,
            agent: row.get(2)?,
            identifier: row.get(3)?,
            team_id: row.get(4)?,
            pin: row.get(5)?,
        },
        label: row.get(6)?,
        location: row.get(7)?,
        cdhash: row.get(8)?,
        caps: row.get::<_, i64>(9)?.try_into().unwrap_or(0),
        created: row.get(10)?,
        last_used: row.get(11)?,
        uses: row.get(12)?,
    })
}

impl Store {
    /// Every stored approval, the most recently used first.
    pub fn agent_grants(&self) -> Result<Vec<AgentGrantRow>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM agent_grants ORDER BY COALESCE(last_used, created) DESC, id DESC"
        ))?;
        Ok(stmt.query_map([], row)?.collect::<rusqlite::Result<_>>()?)
    }

    /// The approval of the program `key` names, if there is one.
    pub fn agent_grant(&self, key: &AgentGrantKey) -> Result<Option<AgentGrantRow>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM agent_grants
             WHERE kind = ?1 AND agent = ?2 AND identifier = ?3 AND team_id = ?4 AND pin = ?5"
        ))?;
        let key = (&key.kind, &key.agent, &key.identifier, &key.team_id, &key.pin);
        Ok(stmt.query_row(params![key.0, key.1, key.2, key.3, key.4], row).optional()?)
    }

    /// Store an approval, or renew the one the same program already has (its capabilities, name and
    /// where it ran from; its history stays). The row's id.
    pub fn save_agent_grant(&mut self, grant: &AgentGrantRow) -> Result<i64> {
        let key = &grant.key;
        exec(
            &self.conn,
            "INSERT INTO agent_grants
                 (kind, agent, identifier, team_id, pin, label, location, cdhash, caps, created)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (kind, agent, identifier, team_id, pin) DO UPDATE SET
                 label = excluded.label, location = excluded.location,
                 cdhash = excluded.cdhash, caps = excluded.caps",
            params![
                key.kind,
                key.agent,
                key.identifier,
                key.team_id,
                key.pin,
                grant.label,
                grant.location,
                grant.cdhash,
                i64::from(grant.caps),
                grant.created,
            ],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM agent_grants
             WHERE kind = ?1 AND agent = ?2 AND identifier = ?3 AND team_id = ?4 AND pin = ?5",
            params![key.kind, key.agent, key.identifier, key.team_id, key.pin],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Note that approval `id` was used at `now` (Unix seconds), where it ran from and its hash. A
    /// caller that does this on every request writes the database on every request: the app does it
    /// at most once a minute for an approval.
    pub fn touch_agent_grant(&mut self, id: i64, now: i64, location: &str, cdhash: &str) -> Result<()> {
        exec(
            &self.conn,
            "UPDATE agent_grants SET last_used = ?2, uses = uses + 1, location = ?3, cdhash = ?4 WHERE id = ?1",
            params![id, now, location, cdhash],
        )?;
        Ok(())
    }

    /// Change what approval `id` allows.
    pub fn set_agent_grant_caps(&mut self, id: i64, caps: u32) -> Result<()> {
        exec(&self.conn, "UPDATE agent_grants SET caps = ?2 WHERE id = ?1", params![id, i64::from(caps)])?;
        Ok(())
    }

    /// Take approval `id` back. Whether there was one.
    pub fn delete_agent_grant(&mut self, id: i64) -> Result<bool> {
        Ok(exec(&self.conn, "DELETE FROM agent_grants WHERE id = ?1", params![id])? > 0)
    }
}
