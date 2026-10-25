//! `probe validate`: the findings, then what the Probe would be allowed to do.

use std::path::Path;

use serde::Serialize;

use super::Outcome;
use super::text::{self, Out, count};
use crate::probe::read::ProbeFolder;
use crate::probe::validate::{
    Combination, Confirmation, DeclaredPermission, Finding, ProgramClass, ProgramNote, Report, Severity,
    Summary, validate,
};

/// The version of the `--json` output of `probe validate` and `probe test`. Adding a field is not a
/// new version; changing or removing one is.
pub(super) const JSON_VERSION: u32 = 1;

pub(super) fn run(dir: &Path, json: bool) -> Outcome {
    let folder = match ProbeFolder::read(dir) {
        Ok(folder) => folder,
        Err(err) => return unreadable(dir, &err.to_string(), json),
    };
    let report = validate(&folder);
    let code = i32::from(!report.is_ok());
    let stdout =
        if json { text::json_line(&Json::of(dir, &folder, &report)) } else { render(dir, &folder, &report) };
    Outcome::new(code, stdout, String::new())
}

/// The folder did not read: no Probe to report on.
pub(super) fn unreadable(dir: &Path, why: &str, json: bool) -> Outcome {
    if json {
        let reply = Unreadable {
            version: JSON_VERSION,
            ok: false,
            folder: dir.to_string_lossy().into_owned(),
            error: why.to_string(),
        };
        return Outcome::new(1, text::json_line(&reply), String::new());
    }
    Outcome::fail(1, &format!("{}: {why}", dir.display()))
}

#[derive(Serialize)]
struct Unreadable {
    version: u32,
    ok: bool,
    folder: String,
    error: String,
}

// ---- text ----------------------------------------------------------------------------------

/// How a permission is confirmed, in words.
fn confirmation(how: Confirmation) -> &'static str {
    match how {
        Confirmation::Click => "Click",
        Confirmation::TouchId => "Touch ID",
    }
}

/// What a program can do, in words.
pub(super) fn class_words(class: ProgramClass) -> &'static str {
    match class {
        ProgramClass::ReadOnlyGit => "read-only git",
        ProgramClass::OwnScript => "own script",
        ProgramClass::RunsAnything => "can run anything",
        ProgramClass::RunsFolderCode => "runs code the folder brings",
        ProgramClass::Other => "other program",
    }
}

fn class_key(class: ProgramClass) -> &'static str {
    match class {
        ProgramClass::ReadOnlyGit => "read_only_git",
        ProgramClass::OwnScript => "own_script",
        ProgramClass::RunsAnything => "runs_anything",
        ProgramClass::RunsFolderCode => "runs_folder_code",
        ProgramClass::Other => "other",
    }
}

fn render(dir: &Path, folder: &ProbeFolder, report: &Report) -> String {
    let info = &folder.manifest.probe;
    let mut out = Out::default();
    out.line(&format!("{} {} in {}", info.id, info.version, text::path(dir)));
    out.line(&format!("  {}", info.name));
    let c = &folder.manifest.contributes;
    let parts: Vec<String> = [
        (c.rules.len(), "rule", "rules"),
        (c.commands.len(), "command", "commands"),
        (c.status.len(), "status item", "status items"),
        (c.agents.len(), "agent", "agents"),
        (c.themes.len(), "theme", "themes"),
        (c.icon_themes.len(), "icon theme", "icon themes"),
    ]
    .into_iter()
    .filter(|(n, ..)| *n > 0)
    .map(|(n, one, many)| count(n, one, many))
    .collect();
    out.line(&format!(
        "  contributes {}",
        if parts.is_empty() { "nothing".to_string() } else { parts.join(", ") }
    ));
    out.blank();

    let errors: Vec<&Finding> = report.errors().collect();
    let warnings: Vec<&Finding> = report.warnings().collect();
    for (title, group) in [("errors", &errors), ("warnings", &warnings)] {
        if group.is_empty() {
            continue;
        }
        out.line(&format!("{title} ({})", group.len()));
        for finding in group {
            let place = if finding.path.is_empty() { "(the folder)" } else { finding.path.as_str() };
            out.line(&format!("  [{}] {place}", finding.code));
            out.line(&format!("      {}", finding.message));
        }
        out.blank();
    }
    summary(&report.summary, &mut out);
    out.blank();
    out.line(&match (errors.len(), warnings.len()) {
        (0, 0) => "ok: no errors and no warnings".to_string(),
        (0, w) => format!("ok: no errors, {}", count(w, "warning", "warnings")),
        (e, w) => format!("{}, {}", count(e, "error", "errors"), count(w, "warning", "warnings")),
    });
    out.finish()
}

/// What the Probe asks for, so that someone can read it before enabling the Probe.
fn summary(summary: &Summary, out: &mut Out) {
    out.line("What it asks for");
    if summary.permissions.is_empty() {
        out.line("  nothing: it runs no program and reaches nothing");
    }
    let width = summary.permissions.iter().map(|p| p.key.len()).max().unwrap_or(0);
    for DeclaredPermission { key, detail, confirmation: how } in &summary.permissions {
        out.line(&format!("  {:<8}  {key:<width$}  {detail}", confirmation(*how)));
    }
    out.blank();
    out.line("Dangerous combinations");
    if summary.combinations.is_empty() {
        out.line("  none");
    }
    for Combination { description } in &summary.combinations {
        out.line(&format!("  ! {description}"));
    }
    if !summary.programs.is_empty() {
        out.blank();
        out.line("Programs it may run");
        let width = summary.programs.iter().map(|p| p.program.chars().count()).max().unwrap_or(0);
        for ProgramNote { program, class } in &summary.programs {
            let pad = width.saturating_sub(program.chars().count());
            out.line(&format!("  {program}{}  {}", " ".repeat(pad), class_words(*class)));
        }
    }
}

// ---- JSON ----------------------------------------------------------------------------------

#[derive(Serialize)]
struct Json<'a> {
    version: u32,
    ok: bool,
    folder: String,
    probe: JsonProbe<'a>,
    counts: Counts,
    findings: Vec<JsonFinding<'a>>,
    summary: JsonSummary<'a>,
}

#[derive(Serialize)]
struct JsonProbe<'a> {
    id: &'a str,
    name: &'a str,
    version: &'a str,
    /// `files` (E1) or `code` (E2).
    level: &'static str,
}

#[derive(Serialize)]
struct Counts {
    errors: usize,
    warnings: usize,
}

#[derive(Serialize)]
struct JsonFinding<'a> {
    severity: &'static str,
    code: &'a str,
    path: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
struct JsonSummary<'a> {
    has_code: bool,
    permissions: Vec<JsonPermission<'a>>,
    combinations: Vec<&'a str>,
    programs: Vec<JsonProgram<'a>>,
}

#[derive(Serialize)]
struct JsonPermission<'a> {
    key: &'a str,
    detail: &'a str,
    /// `click` or `touch_id`.
    confirmation: &'static str,
}

#[derive(Serialize)]
struct JsonProgram<'a> {
    program: &'a str,
    /// `read_only_git`, `own_script`, `runs_anything`, `runs_folder_code` or `other`.
    class: &'static str,
    /// The words the text output uses.
    description: &'static str,
}

impl<'a> Json<'a> {
    fn of(dir: &Path, folder: &'a ProbeFolder, report: &'a Report) -> Self {
        let info = &folder.manifest.probe;
        Self {
            version: JSON_VERSION,
            ok: report.is_ok(),
            folder: dir.to_string_lossy().into_owned(),
            probe: JsonProbe {
                id: &info.id,
                name: &info.name,
                version: &info.version,
                level: if info.main.is_some() { "code" } else { "files" },
            },
            counts: Counts { errors: report.errors().count(), warnings: report.warnings().count() },
            findings: report
                .findings
                .iter()
                .map(|f| JsonFinding {
                    severity: match f.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                    },
                    code: f.code,
                    path: &f.path,
                    message: &f.message,
                })
                .collect(),
            summary: JsonSummary {
                has_code: report.summary.has_code,
                permissions: report
                    .summary
                    .permissions
                    .iter()
                    .map(|p| JsonPermission {
                        key: p.key,
                        detail: &p.detail,
                        confirmation: match p.confirmation {
                            Confirmation::Click => "click",
                            Confirmation::TouchId => "touch_id",
                        },
                    })
                    .collect(),
                combinations: report.summary.combinations.iter().map(|c| c.description.as_str()).collect(),
                programs: report
                    .summary
                    .programs
                    .iter()
                    .map(|p| JsonProgram {
                        program: &p.program,
                        class: class_key(p.class),
                        description: class_words(p.class),
                    })
                    .collect(),
            },
        }
    }
}
