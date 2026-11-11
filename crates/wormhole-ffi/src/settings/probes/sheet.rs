//! What the approval sheet shows of a request: who, what changed since the last approval, what it asks for,
//! each permission with the programs it would run. Every word of a Probe's own passes `reveal`.

use wormhole_core::probe::approval::Request;
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::validate::{Confirmation, ProgramClass};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::words::{argv_line, describe};

/// One permission of the sheet, with its own switch.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProbePermissionRecord {
    /// Its place in the request: what `approve` and `needs_touch_id` take back for each one left on.
    pub index: u32,
    pub words: String,
    /// Approving with it on needs Touch ID.
    pub touch_id: bool,
    pub granted: bool,
    /// What kind of program it runs, when it runs one, and whether that is a warning.
    pub program_note: Option<String>,
    pub program_warns: bool,
    /// The command lines of the Probe that run this program, written out.
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProbeChangeRecord {
    pub text: String,
    /// A permission it did not ask for before.
    pub added: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProbeSheetRecord {
    pub id: String,
    pub title: String,
    pub identity: String,
    pub description: String,
    /// Empty for a first approval.
    pub since: Vec<ProbeChangeRecord>,
    /// "Asks for" or "Asks for nothing".
    pub asks_heading: String,
    pub permissions: Vec<ProbePermissionRecord>,
    pub combinations: Vec<String>,
    pub files_fingerprint: String,
}

pub(super) fn of(request: &Request) -> ProbeSheetRecord {
    let probe = &request.folder().manifest.probe;
    let name = reveal(&probe.name).into_owned();
    let changes = request.changes();
    let title = if changes.is_some() {
        settings_extensions_sheet_title_again(&name)
    } else {
        settings_extensions_sheet_title(&name)
    };
    let mut since = Vec::new();
    if let Some(changes) = &changes {
        if changes.files_only() {
            since.push(ProbeChangeRecord {
                text: text(SETTINGS_EXTENSIONS_FILES_CHANGED).into(),
                added: false,
            });
        }
        since.extend(changes.added.iter().map(|added| ProbeChangeRecord {
            text: settings_extensions_now_asks(&reveal(&describe(added))),
            added: true,
        }));
        since.extend(changes.removed.iter().map(|gone| ProbeChangeRecord {
            text: settings_extensions_no_longer_asks(&reveal(gone)),
            added: false,
        }));
    }
    let summary = request.summary();
    let grants = request.initial_grants();
    let manifest = &request.folder().manifest;
    let permissions = summary
        .permissions
        .iter()
        .enumerate()
        .map(|(at, permission)| {
            let program = permission.detail.as_str();
            let (program_note, program_warns) =
                match (permission.key, summary.programs.iter().find(|n| n.program == program)) {
                    ("process", Some(note)) => match note.class {
                        ProgramClass::RunsAnything => {
                            (Some(settings_extensions_runs_anything(&reveal(program))), true)
                        }
                        ProgramClass::RunsFolderCode => {
                            (Some(settings_extensions_runs_folder_code(&reveal(program))), true)
                        }
                        ProgramClass::ReadOnlyGit => {
                            (Some(text(SETTINGS_EXTENSIONS_READ_ONLY_GIT).to_string()), false)
                        }
                        ProgramClass::OwnScript => {
                            (Some(text(SETTINGS_EXTENSIONS_OWN_SCRIPT).to_string()), false)
                        }
                        ProgramClass::Other => (None, false),
                    },
                    _ => (None, false),
                };
            let commands = if permission.key == "process" {
                manifest
                    .contributes
                    .commands
                    .iter()
                    .filter_map(|c| c.run.as_ref())
                    .chain(manifest.contributes.status.iter().filter_map(|s| s.run.as_ref()))
                    .filter(|argv| argv.first().is_some_and(|first| first == program))
                    .map(|argv| argv_line(argv))
                    .collect()
            } else {
                Vec::new()
            };
            ProbePermissionRecord {
                index: u32::try_from(at).unwrap_or(u32::MAX),
                words: describe(permission),
                touch_id: permission.confirmation == Confirmation::TouchId,
                granted: grants.is_granted(permission),
                program_note,
                program_warns,
                commands,
            }
        })
        .collect::<Vec<_>>();
    ProbeSheetRecord {
        id: request.id().to_string(),
        title,
        identity: settings_extensions_identity(
            &reveal(&probe.id),
            &reveal(&probe.version),
            &reveal(&probe.author),
        ),
        description: reveal(&probe.description).into_owned(),
        since,
        asks_heading: text(if permissions.is_empty() {
            SETTINGS_EXTENSIONS_ASKS_FOR_NOTHING
        } else {
            SETTINGS_EXTENSIONS_ASKS_FOR
        })
        .to_string(),
        permissions,
        combinations: summary
            .combinations
            .iter()
            .map(|c| settings_extensions_combination(&reveal(&c.description)))
            .collect(),
        files_fingerprint: settings_extensions_files_fingerprint(
            &request.file_count().to_string(),
            request.short_fingerprint(),
        ),
    }
}
