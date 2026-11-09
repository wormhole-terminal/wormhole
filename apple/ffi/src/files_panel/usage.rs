//! The Usage tab's reports: what the coding agents last said of their plans' limits, one per agent, kept
//! across restarts in the data folder's database (core's `agents::usage`). Claude Code hands its status
//! line command `rate_limits`; Codex and Gemini arrive through their hooks. The reports are the account's,
//! not a terminal's: the latest report of an agent wins.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::usage::{self, Usage};
use wormhole_core::storage::store::Store;

use crate::error::FfiError;
use crate::files_panel::listener::{PanelListener, Waker};
use crate::profile;

/// One meter: a plan's limit window, as the agent reports it.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PlanLimitRecord {
    pub label: String,
    /// 0 to 100.
    pub used: f64,
    /// Unix seconds.
    pub resets_at: Option<i64>,
}

/// What an agent last reported.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct UsageReportRecord {
    /// The agent's key (`claude`, `codex`, `gemini`, ...).
    pub agent: String,
    pub limits: Vec<PlanLimitRecord>,
    /// Gemini reports the latest response's tokens instead of account limits.
    pub response_tokens: Option<u64>,
    /// Unix seconds.
    pub updated: i64,
}

impl From<&Usage> for UsageReportRecord {
    fn from(report: &Usage) -> Self {
        Self {
            agent: agent_key(report.agent),
            limits: report
                .plan_limits()
                .into_iter()
                .map(|limit| PlanLimitRecord {
                    label: limit.label,
                    used: f64::from(limit.used),
                    resets_at: limit.resets_at,
                })
                .collect(),
            response_tokens: report.response_tokens,
            updated: report.updated,
        }
    }
}

fn agent_key(agent: AgentId) -> String {
    match serde_json::to_value(agent) {
        Ok(Value::String(key)) => key,
        _ => format!("{agent:?}").to_lowercase(),
    }
}

fn agent_of(key: &str) -> Option<AgentId> {
    serde_json::from_value(Value::String(key.to_string())).ok()
}

/// The reports, the database they are kept in and the wake for Swift.
#[derive(uniffi::Object)]
pub struct UsageService {
    reports: Mutex<Vec<Usage>>,
    /// `None` when the database can't be opened: the reports are then kept for this run only.
    store: Mutex<Option<Store>>,
    waker: Waker,
}

impl UsageService {
    pub(crate) fn in_dir(dir: &Path, listener: Arc<dyn PanelListener>) -> Arc<Self> {
        let store =
            Store::open_at(dir).inspect_err(|err| tracing::warn!("usage limits are not kept: {err:#}")).ok();
        let saved = store.as_ref().and_then(|store| store.meta(usage::STORE_KEY).ok().flatten());
        let reports = saved.map(|json| usage::from_json(&json)).unwrap_or_default();
        Arc::new(Self { reports: Mutex::new(reports), store: Mutex::new(store), waker: Waker::new(listener) })
    }

    /// Keeps `report` as its agent's latest, saves it and wakes Swift.
    pub(crate) fn record(&self, report: Usage) {
        let json = {
            let mut all = self.reports.lock().unwrap_or_else(PoisonError::into_inner);
            usage::record(&mut all, report);
            usage::to_json(&all)
        };
        if let Some(store) = self.store.lock().unwrap_or_else(PoisonError::into_inner).as_ref()
            && let Err(err) = store.set_meta(usage::STORE_KEY, &json)
        {
            tracing::warn!("remembering the usage limits: {err:#}");
        }
        self.waker.wake();
    }
}

#[uniffi::export]
impl UsageService {
    /// Fails unless this is an isolated instance of the experiment (the reports are kept in its database).
    #[uniffi::constructor]
    pub fn new(listener: Arc<dyn PanelListener>) -> Result<Arc<Self>, FfiError> {
        Ok(Self::in_dir(&profile::active_dir()?, listener))
    }

    /// The latest report of each agent that has reported, in the order they first did.
    pub fn reports(&self) -> Vec<UsageReportRecord> {
        self.reports
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(UsageReportRecord::from)
            .collect()
    }

    /// Claude Code's status line input (its JSON, as the command reads it from standard input) at `now`
    /// (Unix seconds). False when it carries no limit (an API key, or before the session's first reply).
    pub fn record_status_line(&self, input: String, now: i64) -> bool {
        let Ok(input) = serde_json::from_str::<Value>(&input) else { return false };
        match Usage::from_claude_status(&input, now) {
            Some(report) => {
                self.record(report);
                true
            }
            None => false,
        }
    }

    /// A hook of Codex or Gemini (`agent`'s key, the event's name, the hook's JSON input), with `home` the
    /// agent's own folder (Codex's transcript is read from under it, never from anywhere else).
    pub fn record_hook(&self, agent: String, event: String, input: String, home: String, now: i64) -> bool {
        let (Some(agent), Ok(input)) = (agent_of(&agent), serde_json::from_str::<Value>(&input)) else {
            return false;
        };
        match usage::from_hook(agent, &event, &input, Path::new(&home), now) {
            Some(report) => {
                self.record(report);
                true
            }
            None => false,
        }
    }

    /// A report as the app's hook socket carries it (a `Usage` as JSON). False when it is not one.
    pub fn record_report(&self, json: String) -> bool {
        match serde_json::from_str::<Usage>(&json) {
            Ok(report) => {
                self.record(report);
                true
            }
            Err(_) => false,
        }
    }

    /// Swift read the reports: the next report wakes again.
    pub fn drain(&self) {
        self.waker.rearm();
    }
}
