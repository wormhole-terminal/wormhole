//! What the notification rules read of each terminal beyond its row (the GPUI app's `window/ambient.rs` `TabFacts`):
//! who tells its status (an agent, a program that reports its own, neither) and that one's name, its latest
//! notification in its own words (OSC 9, 99, 777, or `wh notify`), the last command's failure, and the program in
//! front with when it started. `Notifier::observe` turns two looks into banners; this only reads the model.

use std::time::UNIX_EPOCH;

use wormhole_core::notify::Source;

use crate::notify::{FailureRecord, ForegroundRecord, NoticeSourceRecord, ProgramNoticeRecord};
use crate::workspace::Workspace;

/// One terminal's part of a look, by its id.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NoticeFactsRecord {
    pub id: i64,
    pub source: NoticeSourceRecord,
    /// The coding agent's name, or the name of the program that reports its status.
    pub who: Option<String>,
    /// The status is what a program reported about itself (OSC 7501), which anyone may send.
    pub reported: bool,
    pub notice: Option<ProgramNoticeRecord>,
    pub failure: Option<FailureRecord>,
    pub foreground: Option<ForegroundRecord>,
}

#[uniffi::export]
impl Workspace {
    /// Every terminal's notice facts, with times on the caller's monotonic clock `now_ms` (the one it passes
    /// `Notifier::observe`) and the Unix clock.
    pub fn notice_facts(&self, now_ms: u64) -> Vec<NoticeFactsRecord> {
        let shared = self.shared();
        let ws_now = shared.now();
        let epoch_ms = shared.epoch.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64);
        shared.peek_model(|model| {
            model
                .ids()
                .into_iter()
                .filter_map(|id| {
                    let facts = model.facts(id)?;
                    let source = facts.source();
                    let who = match source {
                        Source::Agent => {
                            facts.agent.map(|agent| wormhole_core::agents::agent(agent).name.to_string())
                        }
                        Source::Program => model.program_name(id),
                        Source::Plain => None,
                    };
                    Some(NoticeFactsRecord {
                        id,
                        source: source.into(),
                        who,
                        reported: facts.reports,
                        notice: facts.notice.as_ref().map(|notice| ProgramNoticeRecord {
                            serial: notice.serial,
                            title: notice.title.clone(),
                            body: notice.body.clone(),
                        }),
                        failure: facts.failure.as_ref().map(|failure| FailureRecord {
                            serial: failure.serial,
                            code: failure.code,
                            at_ms: now_ms.saturating_sub(ws_now.saturating_sub(failure.at)),
                            command: failure.command.clone(),
                            took_ms: failure.took,
                            last_line: failure.last_line.clone(),
                        }),
                        foreground: facts.command.clone().zip(facts.running_since).map(|(command, since)| {
                            ForegroundRecord { command, started_unix_ms: epoch_ms + since }
                        }),
                    })
                })
                .collect()
        })
    }
}
