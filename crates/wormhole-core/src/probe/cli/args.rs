//! The words after `probe`.

use crate::probe::scaffold::Kind;

/// What `probe` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Command {
    Validate { dir: Option<String>, json: bool },
    Test { dir: Option<String>, json: bool },
    New { id: String, kind: Kind, dir: Option<String> },
    Help,
}

/// Read `args`; the mistake, in a sentence, when they are not a command.
pub(super) fn parse(args: &[String]) -> Result<Command, String> {
    let Some((verb, rest)) = args.split_first() else {
        return Err("probe needs validate, new or test".into());
    };
    if matches!(verb.as_str(), "help" | "--help" | "-h") {
        return Ok(Command::Help);
    }
    if !matches!(verb.as_str(), "validate" | "new" | "test") {
        return Err(format!("unknown probe command {verb:?} (validate, new, test)"));
    }
    let mut json = false;
    let mut kind = None;
    let mut dir_flag = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut only_words = false;
    let mut words = rest.iter().map(String::as_str);
    while let Some(arg) = words.next() {
        if only_words || !arg.starts_with('-') || arg == "-" {
            positional.push(arg);
            continue;
        }
        // `--kind=agent` is `--kind agent`.
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value)),
            _ => (arg, None),
        };
        let mut value = || inline.or_else(|| words.next()).ok_or_else(|| format!("{flag} needs a value"));
        match flag {
            "--" => only_words = true,
            "--help" | "-h" => return Ok(Command::Help),
            "--json" if verb != "new" && inline.is_none() => json = true,
            "--kind" if verb == "new" => {
                let word = value()?;
                kind = Some(Kind::from_word(word).ok_or_else(|| kind_mistake(word))?);
            }
            "--dir" if verb == "new" => dir_flag = Some(value()?.to_string()),
            other => return Err(format!("unknown option {other} for probe {verb}")),
        }
    }
    match verb.as_str() {
        "new" => {
            let [id] = positional.as_slice() else {
                return Err("probe new takes one id: probe new <id> [--kind …] [--dir <parent>]".into());
            };
            Ok(Command::New { id: (*id).to_string(), kind: kind.unwrap_or(Kind::Rules), dir: dir_flag })
        }
        _ => {
            if positional.len() > 1 {
                return Err(format!("probe {verb} takes one folder, not {}", positional.len()));
            }
            let dir = positional.first().map(|dir| (*dir).to_string());
            if verb == "validate" {
                Ok(Command::Validate { dir, json })
            } else {
                Ok(Command::Test { dir, json })
            }
        }
    }
}

fn kind_mistake(word: &str) -> String {
    let words: Vec<&str> = Kind::ALL.iter().map(|kind| kind.word()).collect();
    if word == "code" {
        return "--kind code is not available yet: Probes with code come with the next level; use one of "
            .to_string()
            + &words.join(", ");
    }
    format!("{word:?} is not a kind of Probe ({})", words.join(", "))
}
