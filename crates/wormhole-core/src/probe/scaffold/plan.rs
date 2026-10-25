//! The files of a new Probe, from the texts in `templates/`.

use std::path::PathBuf;

use super::{Kind, ScaffoldError};
use crate::probe::manifest::{Manifest, is_probe_id};
use crate::probe::read::{MANIFEST_FILE, ProbeFile, ProbeFolder};
use crate::probe::validate::validate;

const HEAD: &str = include_str!("templates/head.toml");
const GUIDE: &str = include_str!("templates/PROBE.md");

/// One file to write.
pub(super) struct Planned {
    /// Relative to the Probe's folder.
    pub path: &'static str,
    pub text: String,
    pub executable: bool,
}

impl Planned {
    fn text(path: &'static str, text: String) -> Self {
        Self { path, text, executable: false }
    }
}

/// The files of the Probe `id` of `kind`, and the folder they make, which must validate.
pub(super) fn build(id: &str, kind: Kind) -> Result<(Vec<Planned>, ProbeFolder), ScaffoldError> {
    if !is_probe_id(id) {
        return Err(ScaffoldError::BadId(id.to_string()));
    }
    let say = Words::new(id, kind);
    let body = match kind {
        Kind::Rules => include_str!("templates/rules.toml"),
        Kind::Command => include_str!("templates/command.toml"),
        Kind::Status => include_str!("templates/status.toml"),
        Kind::Agent => include_str!("templates/agent.toml"),
    };
    let notes = match kind {
        Kind::Rules => include_str!("templates/notes-rules.md"),
        Kind::Command => include_str!("templates/notes-command.md"),
        Kind::Status => include_str!("templates/notes-status.md"),
        Kind::Agent => include_str!("templates/notes-agent.md"),
    };
    let fixtures = match kind {
        Kind::Rules => include_str!("templates/fixtures-rules.toml"),
        Kind::Command => include_str!("templates/fixtures-command.toml"),
        Kind::Status | Kind::Agent => include_str!("templates/fixtures-none.toml"),
    };
    let guide = GUIDE.replace("{{notes}}", notes.trim_end());
    let mut files = vec![
        Planned::text(MANIFEST_FILE, say.fill(&format!("{HEAD}{body}"))),
        Planned::text("PROBE.md", say.fill(&guide)),
        Planned::text("tests/example.toml", say.fill(fixtures)),
    ];
    match kind {
        Kind::Command => files.push(Planned {
            path: "scripts/hello.sh",
            text: say.fill(include_str!("templates/hello.sh")),
            executable: true,
        }),
        Kind::Agent => {
            files.push(Planned::text("assets/icon.svg", say.fill(include_str!("templates/icon.svg"))));
        }
        Kind::Rules | Kind::Status => {}
    }
    files.sort_by_key(|file| file.path);

    let manifest = files
        .iter()
        .find(|file| file.path == MANIFEST_FILE)
        .map(|file| Manifest::parse(&file.text))
        .transpose()
        .map_err(|err| ScaffoldError::Template(err.to_string()))?
        .ok_or_else(|| ScaffoldError::Template("no manifest".into()))?;
    let folder = ProbeFolder {
        root: PathBuf::new(),
        manifest: manifest.manifest,
        warnings: manifest.warnings,
        files: files
            .iter()
            .map(|file| ProbeFile::new(file.path, file.text.as_bytes()).with_executable(file.executable))
            .collect(),
    };
    let report = validate(&folder);
    if report.is_ok() && report.warnings().next().is_none() {
        Ok((files, folder))
    } else {
        Err(ScaffoldError::Invalid(report.findings))
    }
}

/// What the `{{…}}` in a template stand for.
struct Words {
    id: String,
    name: String,
    description: &'static str,
    wormhole: String,
    monogram: String,
}

impl Words {
    fn new(id: &str, kind: Kind) -> Self {
        Self {
            id: id.to_string(),
            name: name_of(id),
            description: match kind {
                Kind::Rules => {
                    "Ticket links, shortened email addresses and hidden keys in the terminal's text."
                }
                Kind::Command => "A palette command, and a commit id in the terminal's text that runs it.",
                Kind::Status => "The current git branch in the status bar.",
                Kind::Agent => "A coding agent for Wormhole to recognise.",
            },
            wormhole: app_range(),
            monogram: monogram_of(id),
        }
    }

    fn fill(&self, template: &str) -> String {
        template
            .replace("{{id}}", &self.id)
            .replace("{{name}}", &self.name)
            .replace("{{description}}", self.description)
            .replace("{{wormhole}}", &self.wormhole)
            .replace("{{monogram}}", &self.monogram)
    }
}

/// `my-probe` as a name: `My probe`.
pub(super) fn name_of(id: &str) -> String {
    let spaced = id.replace('-', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => spaced,
    }
}

/// The app versions the Probe runs on: this one and later.
fn app_range() -> String {
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!(">={major}.{minor}"),
        _ => ">=1.0".to_string(),
    }
}

/// Two capitals for an agent without an icon: the id's first two letters.
pub(super) fn monogram_of(id: &str) -> String {
    let mut letters = id.chars().filter(char::is_ascii_alphabetic).map(|c| c.to_ascii_uppercase());
    match (letters.next(), letters.next()) {
        (Some(a), Some(b)) => format!("{a}{b}"),
        (Some(a), None) => format!("{a}{a}"),
        _ => "AG".to_string(),
    }
}
