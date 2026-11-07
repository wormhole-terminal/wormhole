//! A Probe's hooks, compiled and run over one event as data: does the event satisfy the hook's `match`,
//! and what would its actions come to (`repo-docs/docs/research/probes-hooks-and-fold.md`, section 3).
//!
//! This is what `probe test` runs over its `[[hook_case]]`s and what the app's delivery will call, so a
//! hook behaves in a test as it will on a machine. It delivers nothing, reads no file, spawns nothing and
//! draws nothing: an event goes in, the actions it would take come out ([`Planned`]), and a `run`'s
//! outcome is read by [`CompiledHooks::run_effects`] from what the caller saw the process do.
//!
//! - `matching`: a `match` compiled into tests and run over an event's fields.
//! - `plan`: the actions' words, fold range and argument vector, with the fixed rules of a `run`.
//! - `output`: a `run`'s exit code and stdout as effects, under the closed schema.
//!
//! Must not use anything above `rules`; it reads no file and knows no settings.

mod matching;
mod output;
mod plan;

use std::fmt;

use crate::probe::events::{EventData, MAX_HOOKS, parse_template};
use crate::probe::manifest::{Hook, HookActionKind, Manifest};

pub use output::{Allowed, NotifyText, RunOutcome, StatusUpdate, parse_run_output};
pub use plan::Planned;

use matching::Test;
use plan::{Pieces, Source, Step};

/// A hook and what runs it.
#[derive(Debug, Clone)]
pub struct CompiledHook {
    pub hook: Hook,
    tests: Vec<Test>,
    steps: Vec<Step>,
}

/// What a hook does for one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The event did not satisfy the `match`: the first test that failed.
    Skipped(String),
    /// It did: the actions, in order.
    Actions(Vec<Planned>),
}

impl CompiledHook {
    /// The hook's local id.
    pub fn id(&self) -> &str {
        &self.hook.id
    }

    /// What the hook would do for `event`, whose fields are `data`.
    pub fn plan(&self, data: &EventData) -> Plan {
        let captures = match matching::satisfied(&self.tests, data) {
            Ok(captures) => captures,
            Err(why) => return Plan::Skipped(why),
        };
        let source = Source { data, captures: &captures };
        Plan::Actions(self.steps.iter().map(|step| plan::plan(step, &source)).collect())
    }
}

/// Why a hook cannot be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// The hook's index in `contributes.hooks`.
    pub index: usize,
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contributes.hooks[{}]: {}", self.index, self.message)
    }
}

impl std::error::Error for CompileError {}

/// Every hook of a Probe, ready to run.
#[derive(Debug, Clone, Default)]
pub struct CompiledHooks {
    hooks: Vec<CompiledHook>,
    /// The Probe's `notify = true`: what a `run`'s output may ask for.
    notify: bool,
}

impl CompiledHooks {
    /// Compile the hooks of `manifest` with the limits `validate` checks them against. A Probe that
    /// passed `validate` compiles; one that did not may not, and the first hook that fails is named.
    pub fn compile(manifest: &Manifest) -> Result<Self, CompileError> {
        let hooks = &manifest.contributes.hooks;
        if hooks.len() > MAX_HOOKS {
            return Err(CompileError {
                index: MAX_HOOKS,
                message: format!("a Probe may have {MAX_HOOKS} hooks"),
            });
        }
        let compiled = hooks
            .iter()
            .enumerate()
            .map(|(index, hook)| {
                compile_hook(manifest, hook).map_err(|message| CompileError { index, message })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { hooks: compiled, notify: manifest.permissions.notify })
    }

    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CompiledHook> {
        self.hooks.iter()
    }

    /// The hook with local id `id`.
    pub fn get(&self, id: &str) -> Option<&CompiledHook> {
        self.hooks.iter().find(|h| h.id() == id)
    }

    /// What a `run` that fed `item` (or none) comes to once its process ended with `exit` and printed
    /// `stdout`: the effects, as plan steps, plus the log lines the closed schema gave.
    pub fn run_effects(&self, item: Option<&str>, exit: i32, stdout: &str) -> (Vec<Planned>, Vec<String>) {
        match parse_run_output(exit, stdout, Allowed { status: item.is_some(), notify: self.notify }) {
            RunOutcome::Failed { exit } => {
                let failed = item.map(|item| Planned::Status {
                    item: item.to_string(),
                    text: format!("failed (exit {exit})"),
                    tone: Some("muted".into()),
                    icon: None,
                    tooltip: None,
                });
                (failed.into_iter().collect(), vec![format!("exited with {exit}")])
            }
            RunOutcome::Effects { status, notify, warnings } => {
                let mut planned = Vec::new();
                if let (Some(StatusUpdate { text, icon, tone, tooltip }), Some(item)) = (status, item) {
                    planned.push(Planned::Status { item: item.to_string(), text, tone, icon, tooltip });
                }
                if let Some(NotifyText { title, body }) = notify {
                    planned.push(Planned::Notify { title, body });
                }
                (planned, warnings)
            }
        }
    }
}

fn compile_hook(manifest: &Manifest, hook: &Hook) -> Result<CompiledHook, String> {
    let tests = matching::compile(hook)?;
    let template = |text: &Option<String>| -> Result<Option<Vec<_>>, String> {
        text.as_deref().map(|t| parse_template(t).map_err(|why| format!("`{t}` {why}"))).transpose()
    };
    let steps = hook
        .actions
        .iter()
        .map(|action| {
            let pieces = Pieces {
                title: template(&action.title)?,
                body: template(&action.body)?,
                text: template(&action.text)?,
                tooltip: template(&action.tooltip)?,
                path: template(&action.path)?,
                line: template(&action.line)?,
            };
            let run = match (action.kind, &action.command) {
                (HookActionKind::Run, Some(id)) => {
                    let command = manifest.contributes.commands.iter().find(|c| &c.id == id);
                    command.and_then(|c| c.run.clone().map(|argv| (id.clone(), argv, c.cwd)))
                }
                _ => None,
            };
            Ok(Step { action: action.clone(), pieces, run })
        })
        .collect::<Result<_, String>>()?;
    Ok(CompiledHook { hook: hook.clone(), tests, steps })
}

#[cfg(test)]
mod tests;
