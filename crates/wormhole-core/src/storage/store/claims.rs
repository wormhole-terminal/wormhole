//! The files each terminal changed, kept so that the count on its row survives a restart
//! (`git::attribution` works them out; the app hands them over here).
//!
//! They are file paths, not what a terminal shows, so "Don't keep this terminal's text" does not
//! apply to them: nothing in a path is output of the program.

use anyhow::Result;
use rusqlite::params;

use super::{Store, exec};

/// One file a terminal changed: the repository it is in and the path in it, as `git status` names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimRecord {
    pub session_id: i64,
    pub root: String,
    pub path: String,
}

impl Store {
    /// Make the claims of `sessions` exactly `claims` (those of other sessions are not touched), in
    /// one transaction: the files a terminal no longer has are forgotten, so a commit takes them
    /// off for good.
    pub fn save_claims(&mut self, sessions: &[i64], claims: &[ClaimRecord]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for id in sessions {
            exec(&tx, "DELETE FROM row_claims WHERE session_id = ?1", [id])?;
        }
        for claim in claims {
            exec(
                &tx,
                "INSERT OR IGNORE INTO row_claims (session_id, root, path) VALUES (?1, ?2, ?3)",
                params![claim.session_id, claim.root, claim.path],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The claims of `sessions`, for the terminals that come back after a restart.
    pub fn claims(&self, sessions: &[i64]) -> Result<Vec<ClaimRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT root, path FROM row_claims WHERE session_id = ?1 ORDER BY root, path")?;
        let mut claims = Vec::new();
        for &session_id in sessions {
            let rows = stmt.query_map([session_id], |row| {
                Ok(ClaimRecord { session_id, root: row.get(0)?, path: row.get(1)? })
            })?;
            for row in rows {
                claims.push(row?);
            }
        }
        Ok(claims)
    }

    /// Forget every claim (the setting that keeps them is turned off).
    pub fn clear_claims(&self) -> Result<()> {
        exec(&self.conn, "DELETE FROM row_claims", [])?;
        Ok(())
    }
}
