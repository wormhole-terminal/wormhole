//! A question as the sheet shows it: who asks, for what, the program behind it and what the person may answer.

use std::path::Path;

use wormhole_core::ipc::access::{Capability, Kind, Need, Pending, Resolved, agent_word};
use wormhole_core::platform::codesign::SignerKind;
use wormhole_core::platform::{paths, process};

use super::{AccessDesk, stored_rows};
use crate::aux_backends::access_records::{
    AccessCapability, AccessKindRecord, AccessProgramRecord, AccessQuestionRecord, AccessSignerKind,
};

impl AccessDesk {
    pub(super) fn record_of(&self, pending: &Pending) -> AccessQuestionRecord {
        let resolved = &pending.resolved;
        let capability = match pending.need {
            Need::Capability(Capability::Read) => AccessCapability::Read,
            Need::Capability(Capability::Type) => AccessCapability::Type,
            Need::Capability(Capability::Run) => AccessCapability::Run,
            Need::Capability(Capability::Open) => AccessCapability::Open,
            Need::Capability(Capability::Close) | Need::Approved | Need::Nothing => AccessCapability::Close,
        };
        let kind = match resolved.identity.kind {
            Kind::Scripts => AccessKindRecord::Scripts,
            Kind::Script => AccessKindRecord::Script,
            Kind::Unsigned => AccessKindRecord::Unsigned,
            Kind::AdHoc => AccessKindRecord::AdHoc,
            Kind::DeveloperId => AccessKindRecord::DeveloperId,
            Kind::Apple => AccessKindRecord::Apple,
        };
        let program = resolved.program.as_ref().map(|program| AccessProgramRecord {
            path: shown_path(&program.path),
            pid: program.pid,
            signer: match program.signature.kind {
                SignerKind::DeveloperId => AccessSignerKind::DeveloperId,
                SignerKind::AppleSystem => AccessSignerKind::AppleSystem,
                SignerKind::AdHoc => AccessSignerKind::AdHoc,
                SignerKind::Other => AccessSignerKind::Other,
                SignerKind::Unsigned => AccessSignerKind::Unsigned,
                SignerKind::Unreadable => AccessSignerKind::Unreadable,
            },
            subject: program.signature.subject.clone(),
            team: program.signature.team_id.clone(),
            interpreter: program.interpreter.clone(),
            started_by: resolved.started_by.clone().or_else(|| parent_name(program.pid)),
        });
        AccessQuestionRecord {
            id: pending.id,
            label: resolved.label.clone(),
            capability,
            kind,
            program,
            about_terminal: pending.about,
            unusual_signer: resolved.unusual_signer,
            changed_since_allowed: self.changed_since_allowed(resolved),
            elevated: resolved.elevated,
            can_be_remembered: resolved.identity.can_be_remembered() && pending.about.is_none(),
        }
    }

    /// Whether the same agent was allowed before as something else: another signer, another build, another
    /// install (stored approvals only: what "for this session" allowed ended with the process).
    fn changed_since_allowed(&self, resolved: &Resolved) -> bool {
        let Some(agent) = resolved.identity.agent else { return false };
        let key = resolved.identity.key();
        let word = agent_word(agent);
        stored_rows(&self.profile_dir)
            .is_some_and(|rows| rows.iter().any(|r| r.key.agent == word && r.key != key))
    }
}

/// A path as the title row writes it: the home folder as `~`.
fn shown_path(path: &Path) -> String {
    match paths::home().as_deref().and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The program that started `pid`, by name.
fn parent_name(pid: u32) -> Option<String> {
    let path = process::executable(process::parent(pid)?)?;
    path.file_name().map(|name| name.to_string_lossy().into_owned())
}
