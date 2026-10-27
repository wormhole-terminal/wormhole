//! The guards that keep what agents are taught from drifting away from what the program does: the
//! skill (`agents::skill`) names commands, flags, tools and codes, and each of them is held here to the
//! parser, the MCP server and the code tables. They sit above `control` and `mcp` because they read both.

use std::path::Path;

use serde_json::Value;

use super::control::cli::verbs::{GLOBAL_FLAGS, TARGET_FLAGS, VERBS};
use super::control::{self, ErrorCode, Parsed, Reason, help_json, parse, sessions, skill_cli};
use super::mcp;
use crate::agents::skill::{FILES, file};

/// Split a shell line into words as a shell would, enough for the examples: quotes group, and an
/// unquoted `|`, `&&`, `||`, `;` or ` #` ends the command.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut has_word = false;
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                has_word = true;
            }
            (None, ' ' | '\t') => {
                if has_word {
                    out.push(std::mem::take(&mut word));
                    has_word = false;
                }
            }
            (None, '#') if !has_word => break,
            (None, '|' | ';' | '&') => break,
            (None, c) => {
                word.push(c);
                has_word = true;
            }
        }
    }
    assert!(quote.is_none(), "an unclosed quote in {line:?}");
    if has_word {
        out.push(word);
    }
    out
}

/// Every `wh …` line inside a fenced code block of the skill's files, with where it is.
fn examples() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for file in FILES {
        let mut inside = false;
        for line in file.text.lines() {
            if line.trim_start().starts_with("```") {
                inside = !inside;
            } else if inside && line.trim_start().starts_with("wh ") {
                found.push((file.path.to_string(), line.trim().to_string()));
            }
        }
    }
    found
}

/// Whether the command line reads `line`'s arguments, by whoever reads that verb.
fn reads(args: &[String]) -> Result<(), String> {
    let Some((verb, rest)) = args.split_first() else { return Err("no verb".into()) };
    match verb.as_str() {
        "skill" => skill_cli::parse(rest).map(drop),
        "sessions" => sessions::parse(rest).map(drop),
        "mcp" if rest.is_empty() => Ok(()),
        "mcp" => Err("mcp takes no arguments".into()),
        "welcome" => Ok(()),
        _ => match parse(args, Path::new("/work"), Some(Path::new("/home/me"))) {
            Ok(Parsed::Run(_) | Parsed::Help | Parsed::HelpJson) => Ok(()),
            Err(mistake) => Err(mistake),
        },
    }
}

#[test]
fn every_command_in_the_skill_parses() {
    let examples = examples();
    assert!(examples.len() > 30, "the examples were found: {}", examples.len());
    let mut bad = Vec::new();
    for (path, line) in &examples {
        let args = words(line);
        assert_eq!(args.first().map(String::as_str), Some("wh"));
        if let Err(why) = reads(&args[1..]) {
            bad.push(format!("{path}: {line}\n    {why}"));
        }
    }
    assert!(bad.is_empty(), "lines the command line does not read:\n{}", bad.join("\n"));
}

#[test]
fn the_word_splitter_reads_the_examples_the_way_a_shell_does() {
    assert_eq!(
        words("wh run --terminal 9 'cargo test' --timeout 900 --json"),
        ["wh", "run", "--terminal", "9", "cargo test", "--timeout", "900", "--json"]
    );
    assert_eq!(
        words("wh read --terminal 3 --last 5 # five lines"),
        ["wh", "read", "--terminal", "3", "--last", "5"]
    );
    assert_eq!(
        words("wh send-text --terminal 9 --submit 'a | b; echo $?' | tail"),
        ["wh", "send-text", "--terminal", "9", "--submit", "a | b; echo $?"]
    );
}

fn data() -> Value {
    serde_json::from_str::<Value>(help_json().trim()).expect("help --json is JSON")["result"].clone()
}

#[test]
fn every_verb_is_in_the_skill_and_every_flag_in_the_commands_reference() {
    let everything: String = FILES.iter().map(|f| f.text).collect();
    let commands = file("references/commands.md").expect("the commands reference").text;
    let data = data();
    for verb in data["verbs"].as_array().expect("verbs") {
        let name = verb["name"].as_str().expect("name");
        assert!(everything.contains(&format!("wh {name}")), "the skill never mentions `wh {name}`");
        for flag in verb["flags"].as_array().expect("flags") {
            let flag = flag["name"].as_str().expect("flag");
            assert!(commands.contains(flag), "references/commands.md never mentions {flag} (of {name})");
        }
    }
}

#[test]
fn every_mcp_tool_is_in_the_mcp_reference() {
    let reference = file("references/mcp.md").expect("the MCP reference").text;
    let names = mcp::tool_names();
    assert!(names.len() >= 15, "{names:?}");
    for name in names {
        assert!(
            reference.contains(&format!("`{name}`")),
            "references/mcp.md never mentions the tool `{name}`"
        );
    }
}

#[test]
fn every_exit_code_error_code_and_reason_is_in_the_errors_reference() {
    let reference = file("references/errors.md").expect("the errors reference").text;
    for (code, _) in control::cli::verbs::EXIT_CODES {
        assert!(reference.contains(&format!("| {code} |")), "exit code {code}");
    }
    for code in ErrorCode::ALL {
        assert!(reference.contains(&format!("`{}`", code.word())), "error code {}", code.word());
    }
    for reason in Reason::ALL {
        assert!(reference.contains(&format!("`{}`", reason.word())), "reason {}", reason.word());
    }
}

/// What a flag's value looks like in an example.
fn sample(value: &str) -> &'static str {
    match value {
        "<from>:<to>" => "0:5",
        "<id>|last" => "last",
        "left|middle|right" => "left",
        "<state>[,<state>]" => "idle",
        "<list>" => "status",
        "<id>" | "<n>" | "<s>" | "<ms>" | "<block>" | "<percent>" => "5",
        _ => "x",
    }
}

#[test]
fn every_flag_in_the_table_is_read_by_the_parser() {
    for verb in VERBS.iter().filter(|verb| !verb.offline) {
        for flag in verb.flags {
            let mut args = vec![verb.name.to_string(), "--terminal".into(), "3".into()];
            args.push(flag.name.to_string());
            if let Some(value) = flag.value {
                args.push(sample(value).to_string());
            }
            if let Err(mistake) = parse(&args, Path::new("/work"), None) {
                assert!(!mistake.contains("unknown option"), "{} {}: {mistake}", verb.name, flag.name);
            }
        }
    }
    for flag in GLOBAL_FLAGS.iter().chain(TARGET_FLAGS) {
        let mut args = vec!["status".to_string(), flag.name.to_string()];
        args.extend(flag.value.map(|value| sample(value).to_string()));
        if let Err(mistake) = parse(&args, Path::new("/work"), None) {
            assert!(!mistake.contains("unknown option"), "{}: {mistake}", flag.name);
        }
    }
}

/// The parser and the table name the same flags: a flag added to one without the other fails here.
#[test]
fn the_parser_reads_exactly_the_flags_the_table_lists() {
    let source = include_str!("control/cli/parse.rs");
    let source = &source[..source.find("#[cfg(test)]").unwrap_or(source.len())];
    let mut read: Vec<String> = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("\"--") {
        rest = &rest[at + 1..];
        let end = rest.find(|c: char| !(c == '-' || c.is_ascii_lowercase())).unwrap_or(rest.len());
        read.push(rest[..end].to_string());
    }
    read.sort();
    read.dedup();
    let mut table: Vec<String> = VERBS
        .iter()
        .filter(|verb| !verb.offline)
        .flat_map(|verb| verb.flags.iter().map(|flag| flag.name.to_string()))
        .chain(GLOBAL_FLAGS.iter().chain(TARGET_FLAGS).map(|flag| flag.name.to_string()))
        .collect();
    table.sort();
    table.dedup();
    for flag in &read {
        assert!(
            table.contains(flag),
            "parse.rs reads {flag}, which the verbs table (cli/verbs.rs) does not list"
        );
    }
    for flag in &table {
        assert!(read.contains(flag), "the verbs table lists {flag}, which parse.rs does not read");
    }
}

#[test]
fn every_verb_the_parser_knows_is_in_the_table_and_the_other_way_round() {
    use control::is_verb;
    for verb in VERBS {
        assert!(is_verb(verb.name), "{}", verb.name);
    }
    let source = include_str!("control/cli/parse.rs");
    let source = &source[..source.find("#[cfg(test)]").unwrap_or(source.len())];
    let action = &source[source.find("let action = match verb {").expect("the verb match")..];
    let action = &action[..action.find("other => return Err").expect("its end")];
    for line in action.lines().filter(|line| line.starts_with("        \"")) {
        let name = &line.trim_start()[1..line.trim_start()[1..].find('"').map_or(0, |end| end + 1)];
        assert!(VERBS.iter().any(|verb| verb.name == name), "parse.rs handles {name}, which the table lacks");
    }
}
