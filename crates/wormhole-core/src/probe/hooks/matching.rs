//! Whether an event satisfies a hook's `match`: each field compiled once into a test, then run over the
//! event's fields.

use regex::{Regex, RegexBuilder};

use crate::probe::events::{self, Comparison, EventData, Glob, MatcherKind, STATES, Value, list};
use crate::probe::manifest::Hook;
use crate::probe::rules::scanned;
use crate::probe::validate::SIZE_LIMIT;

/// One field of a `match`.
#[derive(Debug, Clone)]
pub(super) struct Test {
    /// How it reads in a sentence: `exit != 0`.
    words: String,
    kind: Kind,
}

#[derive(Debug, Clone)]
enum Kind {
    Number { field: String, cmp: Comparison },
    In { field: String, allowed: Vec<String> },
    Globs { field: String, globs: Vec<Glob> },
    Pattern { on: &'static str, regex: Regex },
    Front,
    Terminal(String),
}

/// The tests of `hook`, in the order of its fields; an error names the field that does not compile.
pub(super) fn compile(hook: &Hook) -> Result<Vec<Test>, String> {
    let spec = events::spec(hook.on);
    let mut tests = Vec::new();
    for (field, value) in &hook.matcher {
        let matcher = spec
            .matcher(field)
            .ok_or_else(|| format!("`match.{field}` is not a matcher field of `{}`", hook.on.name()))?;
        let bad = |why: String| format!("`match.{field}`: {why}");
        let kind = match matcher.kind {
            MatcherKind::Number => {
                Kind::Number { field: field.clone(), cmp: Comparison::parse(value).map_err(bad)? }
            }
            MatcherKind::States => {
                let allowed = list(value).map_err(bad)?;
                if let Some(state) = allowed.iter().find(|s| !STATES.contains(s)) {
                    return Err(format!("`match.{field}`: `{state}` is not a state"));
                }
                Kind::In { field: field.clone(), allowed: allowed.into_iter().map(String::from).collect() }
            }
            MatcherKind::Names => Kind::In {
                field: field.clone(),
                allowed: list(value).map_err(bad)?.into_iter().map(String::from).collect(),
            },
            MatcherKind::Globs => Kind::Globs {
                field: field.clone(),
                globs: list(value)
                    .map_err(bad)?
                    .into_iter()
                    .map(Glob::parse)
                    .collect::<Result<_, _>>()
                    .map_err(|why| format!("`match.{field}`: {why}"))?,
            },
            MatcherKind::Pattern { on } => Kind::Pattern {
                on,
                regex: RegexBuilder::new(value)
                    .size_limit(SIZE_LIMIT)
                    .build()
                    .map_err(|err| format!("`match.{field}`: {err}"))?,
            },
            MatcherKind::Front => Kind::Front,
            MatcherKind::TerminalKind => Kind::Terminal(value.clone()),
        };
        let words = match &kind {
            Kind::Number { cmp, .. } => format!("{field} {cmp}"),
            Kind::Pattern { .. } => format!("{field} /{value}/"),
            _ => format!("{field} {value}"),
        };
        tests.push(Test { words, kind });
    }
    Ok(tests)
}

/// The captures of the hook's `pattern` (the whole match first) when `event` satisfies every test, or
/// why not: the first test that failed. No pattern, no captures.
pub(super) fn satisfied(tests: &[Test], data: &EventData) -> Result<Vec<String>, String> {
    let mut captures = Vec::new();
    for test in tests {
        let ok = match &test.kind {
            Kind::Number { field, cmp } => data.number(field).is_some_and(|n| cmp.test(n)),
            Kind::In { field, allowed } => data.text(field).is_some_and(|v| allowed.iter().any(|a| a == v)),
            Kind::Globs { field, globs } => {
                data.text(field).is_some_and(|v| globs.iter().any(|g| g.is_match(v)))
            }
            Kind::Pattern { on, regex } => match data.text(on).and_then(|text| regex.captures(scanned(text)))
            {
                Some(found) => {
                    captures =
                        found.iter().map(|m| m.map_or(String::new(), |m| m.as_str().to_string())).collect();
                    true
                }
                None => false,
            },
            Kind::Front => data.front(),
            Kind::Terminal(want) => data.text("kind") == Some(want.as_str()),
        };
        if !ok {
            return Err(format!("`{}` did not hold{}", test.words, seen(test, data)));
        }
    }
    Ok(captures)
}

/// What the event had where a test looked, for the message.
fn seen(test: &Test, data: &EventData) -> String {
    let field = match &test.kind {
        Kind::Number { field, .. } | Kind::In { field, .. } | Kind::Globs { field, .. } => field.as_str(),
        Kind::Pattern { on, .. } => on,
        Kind::Front => "front",
        Kind::Terminal(_) => "kind",
    };
    match data.get(field) {
        Some(Value::Number(n)) => format!(" (`{field}` was {n})"),
        Some(Value::Text(text)) => format!(" (`{field}` was {:?})", scanned(text)),
        None => format!(" (the event has no `{field}`)"),
    }
}
