//! The guards that keep what agents are taught from drifting away from what the program does: the
//! skill (`agents::skill`) names commands, flags, tools and codes, and each of them is held here to the
//! parser, the MCP server and the code tables. They sit above `control` and `mcp` because they read both.

use std::path::Path;

use serde_json::Value;

use super::control::cli::verbs::{GLOBAL_FLAGS, TARGET_FLAGS, Target, VERBS};
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

/// Every `wh …` command in the skill's files, with where it is and whether it is a whole command (a fenced line): the lines of fenced code blocks, and the
/// inline `code` spans of prose and tables that start with `wh `. A span that is a usage form rather than a
/// command (it has `<placeholders>`, `[options]`, `a|b` or `...`) is not one a shell could run, so it is
/// not held to the parser; a command with real values is.
fn examples() -> Vec<(String, String, bool)> {
    let mut found = Vec::new();
    for file in FILES {
        let mut inside = false;
        for line in file.text.lines() {
            if line.trim_start().starts_with("```") {
                inside = !inside;
            } else if inside {
                if line.trim_start().starts_with("wh ") {
                    found.push((file.path.to_string(), line.trim().to_string(), true));
                }
            } else {
                for (at, span) in line.split('`').enumerate() {
                    // The odd pieces are between backticks.
                    let usage = span.contains(['<', '[', '|', '…']) || span.contains("...");
                    if at % 2 == 1 && span.starts_with("wh ") && !usage {
                        found.push((file.path.to_string(), span.to_string(), false));
                    }
                }
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
    assert!(examples.len() > 60, "the examples were found: {}", examples.len());
    // Both kinds: fenced lines and inline spans (a table cell, a sentence).
    assert!(
        examples
            .iter()
            .any(|(path, line, _)| path == "references/commands.md" && line.starts_with("wh doctor"))
    );
    assert!(examples.iter().any(|(_, _, whole)| !whole) && examples.iter().any(|(_, _, whole)| *whole));
    let mut bad = Vec::new();
    for (path, line, whole) in &examples {
        let args = words(line);
        assert_eq!(args.first().map(String::as_str), Some("wh"));
        // A span in a sentence may name a verb or a flag without what it needs (`wh close`, `wh wait
        // --match`): lacking something is no fault there, a flag or a value that is wrong is.
        if let Err(why) = reads(&args[1..])
            .map_err(|why| (why, *whole))
            .or_else(|(why, whole)| if !whole && why.contains(" needs ") { Ok(()) } else { Err(why) })
        {
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

/// The flags each verb's arms in `parse.rs` read, from the arms' own guards: `"--flag" | "--other" if verb ==
/// "x" =>`, `if matches!(verb, "a" | "b")`, `if opens(verb)`; an arm without a guard is every verb's. Only
/// the heads of match arms are read, so a flag named in an error message is not one the parser reads.
fn flags_by_verb() -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
    use std::collections::{BTreeMap, BTreeSet};
    let source = include_str!("control/cli/parse.rs");
    let source = &source[..source.find("#[cfg(test)]").unwrap_or(source.len())];
    let quoted = |text: &str| -> Vec<String> {
        text.split('"').enumerate().filter(|(at, _)| at % 2 == 1).map(|(_, word)| word.to_string()).collect()
    };
    // `fn opens`: the verbs that start a terminal.
    let opens_at = source.find("fn opens(verb: &str)").expect("fn opens");
    let opens =
        quoted(&source[opens_at..source[opens_at..].find('}').map_or(source.len(), |end| opens_at + end)]);
    let mut by_verb: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut everywhere: BTreeSet<String> = BTreeSet::new();
    let mut target: BTreeSet<String> = BTreeSet::new();
    // The arms of the one match over the arguments, at its indentation: a match inside an arm (the side a
    // `--right` means) is not another flag.
    let arm = format!("{}\"-", " ".repeat(12));
    for line in source.lines().filter(|line| line.starts_with(&arm)).map(str::trim) {
        if !line.contains(" =>") {
            continue;
        }
        let head = line.split(" =>").next().expect("a head");
        let (patterns, guard) = head.split_once(" if ").map_or((head, None), |(p, g)| (p, Some(g)));
        let flags: Vec<String> = quoted(patterns).into_iter().filter(|flag| flag != "-h").collect();
        match guard {
            // The one unguarded arm that names a terminal is the target's, for the verbs that take one.
            None if flags.iter().any(|flag| flag == "--terminal") => target.extend(flags),
            None => everywhere.extend(flags),
            Some(guard) => {
                let verbs = if guard.starts_with("opens(") { opens.clone() } else { quoted(guard) };
                for verb in verbs {
                    by_verb.entry(verb).or_default().extend(flags.iter().cloned());
                }
            }
        }
    }
    // `--terminal` is guarded for `events` (which repeats it) and unguarded for the verbs that take a target.
    for verb in VERBS.iter().filter(|verb| !verb.offline) {
        let entry = by_verb.entry(verb.name.to_string()).or_default();
        entry.extend(everywhere.iter().cloned());
        if verb.target != Target::No {
            entry.extend(target.iter().cloned());
        }
    }
    by_verb
}

/// The parser and the table name the same flags, verb by verb: a flag added to one verb in one and not the
/// other fails here, and so does a flag that the table gives to another verb than the parser does.
#[test]
fn the_parser_reads_exactly_the_flags_the_table_lists_for_each_verb() {
    use std::collections::BTreeSet;
    let read = flags_by_verb();
    for verb in VERBS.iter().filter(|verb| !verb.offline) {
        let mut listed: BTreeSet<String> = verb.flags.iter().map(|flag| flag.name.to_string()).collect();
        listed.extend(GLOBAL_FLAGS.iter().map(|flag| flag.name.to_string()));
        if verb.target != Target::No {
            listed.extend(TARGET_FLAGS.iter().map(|flag| flag.name.to_string()));
        }
        assert_eq!(
            read.get(verb.name).cloned().unwrap_or_default(),
            listed,
            "the flags of `{}` in parse.rs (left) and in cli/verbs.rs (right)",
            verb.name
        );
    }
}

/// The offline verbs read their own arguments: each flag of theirs in the table is one they take.
#[test]
fn the_offline_verbs_read_the_flags_the_table_lists_for_them() {
    let skill = VERBS.iter().find(|verb| verb.name == "skill").expect("skill");
    for flag in skill.flags {
        let mut args = vec!["show".to_string(), flag.name.to_string()];
        args.extend(flag.value.map(|value| if value == "<id>" { "claude" } else { "x" }.to_string()));
        let mistake = skill_cli::parse(&args).err().unwrap_or_default();
        assert!(!mistake.contains("unknown option"), "skill {}: {mistake}", flag.name);
    }
    let sessions_verb = VERBS.iter().find(|verb| verb.name == "sessions").expect("sessions");
    for flag in sessions_verb.flags {
        let mistake = sessions::parse(&["end".into(), flag.name.to_string()]).err().unwrap_or_default();
        assert!(!mistake.contains("unknown option"), "sessions {}: {mistake}", flag.name);
    }
    // Their subcommands are the ones the table names.
    for name in skill.subcommands {
        assert!(
            !skill_cli::parse(&[name.to_string()])
                .err()
                .unwrap_or_default()
                .contains("unknown skill command")
        );
    }
    for name in sessions_verb.subcommands {
        assert!(
            !sessions::parse(&[name.to_string()])
                .err()
                .unwrap_or_default()
                .contains("unknown sessions command")
        );
    }
}

/// The agents the table's `--agent` names are the agents that load skills, no more and no fewer.
#[test]
fn the_table_names_the_agents_that_load_skills() {
    use crate::agents::skill::TARGETS;
    let skill = VERBS.iter().find(|verb| verb.name == "skill").expect("skill");
    let agent = skill.flags.iter().find(|flag| flag.name == "--agent").expect("--agent");
    let words: Vec<String> = TARGETS.iter().map(|target| target.word()).collect();
    for word in &words {
        assert!(agent.meaning.contains(word.as_str()), "--agent's meaning never names {word}");
    }
    let named = agent.meaning.split('(').nth(1).and_then(|rest| rest.split(')').next()).expect("a list");
    assert_eq!(named.split(", ").count(), words.len(), "{named}");
}

/// Every number of a budget the skill states is the app's: formatted from `Limited::budget`, so a
/// budget that changes fails here until the skill is corrected.
#[test]
fn the_budgets_in_the_skill_are_the_apps() {
    use control::Limited;
    let skill = file("SKILL.md").expect("SKILL.md").text;
    let said = [
        (Limited::NewWindow, "new windows"),
        (Limited::NewTerminal, "new terminals"),
        (Limited::SendText, "`send-text` calls"),
        (Limited::Run, "`run`s"),
        (Limited::SendKeys, "`send-keys` calls"),
        (Limited::Wait, "`wait`s"),
        (Limited::Read, "reads"),
        (Limited::Close, "`close`s"),
    ];
    for (kind, what) in said {
        let phrase = format!("{} {what}", kind.budget());
        assert!(skill.contains(&phrase), "SKILL.md never says \"{phrase}\"");
    }
    // What a key press counts is the schema's: 64 keys to a call.
    assert!(skill.contains(&format!("up to {} keys each", control::keys::KEYS_MAX_ITEMS)));
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
