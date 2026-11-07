//! What the auxiliary windows (update, session hosts, project trust, agent access, first run, licenses) need
//! from the core crates, behind UniFFI. Each file is one object; core is called, never re-done, and every
//! object that reads or writes the data folder refuses to run outside the experiment's own (`common`).

mod access;
mod access_records;
mod common;
mod first_run;
mod hosts;
mod hosts_words;
mod imports;
mod licenses;
mod trust;
mod update_records;
mod updater;

#[cfg(test)]
mod tests;

pub use access::{AccessApprovalRecord, AccessDesk, AccessHow};
pub use access_records::{
    AccessAnswer, AccessCapability, AccessHeldRecord, AccessKindRecord, AccessProgramRecord,
    AccessQuestionRecord, AccessSignerKind,
};
pub(crate) use common::resolve_data_dir;
pub use common::{AuxError, AuxListener};
pub use first_run::{
    AgentScanRecord, FirstRun, FoundAgentRecord, HookState, LookRecord, PlaceKindRecord, PlaceRecord,
};
pub use hosts::{
    HostAttachmentRecord, HostCardRecord, HostFactRecord, HostFieldRecord, HostProgramKind,
    HostStandingRecord, HostToneRecord, HostsDesk, HostsReportRecord,
};
pub use imports::{ImportFoundRecord, ImportRowRecord, import_apply, import_scan};
pub use licenses::{LicensePackageRecord, LicensesRecord, licenses};
pub use trust::{ReviewLineRecord, ReviewStatusRecord, TrustDesk, TrustReviewRecord};
pub use update_records::{
    UpdateChannelNote, UpdateChannelRecord, UpdatePhaseRecord, UpdateReleaseRecord, UpdateSetup,
    UpdateStateRecord,
};
pub use updater::UpdateEngine;
