//! The rules of a manifest, compiled once.

use std::fmt;

use regex::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};

use crate::probe::manifest::{Manifest, Rule};
use crate::probe::validate::{MAX_RULES, SIZE_LIMIT};

/// A rule and the program that finds it.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// The manifest's rule: its look, tone, hover and action stay with it.
    pub rule: Rule,
    pub(super) regex: Regex,
    /// The `run` of the command a `command` action names, when it is there to read.
    pub(super) command_run: Option<Vec<String>>,
}

impl CompiledRule {
    #[cfg(test)]
    pub(super) fn regex_for_tests(&self) -> &Regex {
        &self.regex
    }

    /// The rule's local id (`ticket`, not `acme-tools/ticket`).
    pub fn id(&self) -> &str {
        &self.rule.id
    }
}

/// Why a rule cannot be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// The rule's index in `contributes.rules`.
    pub index: usize,
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contributes.rules[{}].pattern: {}", self.index, self.message)
    }
}

impl std::error::Error for CompileError {}

/// Every rule of a Probe, ready to match.
#[derive(Debug, Clone)]
pub struct CompiledRules {
    pub(super) rules: Vec<CompiledRule>,
    /// Which rules match somewhere in a line, so a line that nothing matches costs one pass. `None`
    /// when the set would not compile within the limit; each rule is then simply tried in turn.
    pub(super) prefilter: Option<RegexSet>,
}

impl CompiledRules {
    /// Compile the rules of `manifest` with the limits `validate` checks them against. A Probe that
    /// passed `validate` compiles; one that did not may not, and the first rule that fails is named.
    pub fn compile(manifest: &Manifest) -> Result<Self, CompileError> {
        let rules = &manifest.contributes.rules;
        if rules.len() > MAX_RULES {
            return Err(CompileError {
                index: MAX_RULES,
                message: format!("a Probe may have {MAX_RULES} rules"),
            });
        }
        let mut compiled = Vec::with_capacity(rules.len());
        for (index, rule) in rules.iter().enumerate() {
            let regex = RegexBuilder::new(&rule.pattern)
                .size_limit(SIZE_LIMIT)
                .build()
                .map_err(|err| CompileError { index, message: err.to_string() })?;
            let command_run = rule
                .action
                .as_ref()
                .and_then(|action| action.command.as_deref())
                .and_then(|id| manifest.contributes.commands.iter().find(|command| command.id == id))
                .and_then(|command| command.run.clone());
            compiled.push(CompiledRule { rule: rule.clone(), regex, command_run });
        }
        // The programs are the prefilter's too; its limit is what all of them may take together.
        let prefilter = (!compiled.is_empty())
            .then(|| {
                RegexSetBuilder::new(compiled.iter().map(|rule| rule.regex.as_str()))
                    .size_limit(SIZE_LIMIT.saturating_mul(compiled.len()))
                    .build()
                    .ok()
            })
            .flatten();
        Ok(Self { rules: compiled, prefilter })
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The rule at `index`, the index a [`Match`](super::Match) carries.
    pub fn rule(&self, index: usize) -> Option<&CompiledRule> {
        self.rules.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CompiledRule> {
        self.rules.iter()
    }
}
