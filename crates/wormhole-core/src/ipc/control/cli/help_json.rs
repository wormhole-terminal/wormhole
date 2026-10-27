//! `wh help --json`: the command line as data for a program that wants to know what exists, built from
//! the tables the parser is held to (`verbs`, the codes) so there is no second list to keep.

use serde_json::{Value, json};

use super::COMMAND_NAME;
use super::verbs::{EXIT_CODES, Flag, GLOBAL_FLAGS, TARGET_FLAGS, Target, VERBS, Verb};
use crate::ipc::control::{ErrorCode, PROTOCOL_VERSION, Reason};

fn flag(flag: &Flag) -> Value {
    let mut out = json!({ "name": flag.name, "meaning": flag.meaning });
    if let Some(value) = flag.value {
        out["value"] = json!(value);
    }
    if flag.repeats {
        out["repeats"] = json!(true);
    }
    if let Some(need) = flag.needs {
        out["needs"] = json!(need.word());
    }
    out
}

fn verb(verb: &Verb) -> Value {
    let target = match verb.target {
        Target::No => "none",
        Target::Optional => "optional",
        Target::Required => "required",
    };
    let mut out = json!({
        "name": verb.name,
        "usage": format!("{COMMAND_NAME} {} {}", verb.name, verb.args).trim_end(),
        "does": verb.does,
        "target": target,
        "needs": verb.needs.iter().map(|need| need.word()).collect::<Vec<_>>(),
        "flags": verb.flags.iter().map(flag).collect::<Vec<_>>(),
        "works_without_the_app": verb.offline,
    });
    if !verb.subcommands.is_empty() {
        out["subcommands"] = json!(verb.subcommands);
    }
    out
}

/// The data, as a reply's `result`.
pub(super) fn data() -> Value {
    json!({
        "command": COMMAND_NAME,
        "protocol": PROTOCOL_VERSION,
        "json": { "flag": "--json", "environment": "WORMHOLE_JSON" },
        "global_flags": GLOBAL_FLAGS.iter().map(flag).collect::<Vec<_>>(),
        "target_flags": TARGET_FLAGS.iter().map(flag).collect::<Vec<_>>(),
        "verbs": VERBS.iter().map(verb).collect::<Vec<_>>(),
        "exit_codes": EXIT_CODES.iter().map(|(code, meaning)| json!({ "code": code, "meaning": meaning })).collect::<Vec<_>>(),
        "error_codes": ErrorCode::ALL
            .iter()
            .map(|code| json!({ "code": code.word(), "exit": code.exit_code(), "advice": code.advice() }))
            .collect::<Vec<_>>(),
        "reasons": Reason::ALL
            .iter()
            .map(|reason| json!({ "reason": reason.word(), "advice": reason.advice() }))
            .collect::<Vec<_>>(),
    })
}

/// `wh help --json`: one line, a reply as every command's `--json` is.
pub fn help_json() -> String {
    format!("{}\n", super::super::printable::json_line(&super::super::Reply::ok(data())))
}

#[cfg(test)]
mod tests {
    use super::data;

    #[test]
    fn the_data_names_every_verb_with_its_flags_and_the_codes() {
        let data = data();
        let verbs = data["verbs"].as_array().expect("verbs");
        let wait = verbs.iter().find(|v| v["name"] == "wait").expect("wait");
        assert_eq!(wait["target"], "required");
        let flags: Vec<_> =
            wait["flags"].as_array().expect("flags").iter().map(|f| f["name"].clone()).collect();
        assert!(flags.contains(&serde_json::json!("--block-done")));
        assert!(data["error_codes"].as_array().expect("codes").iter().any(|c| c["code"] == "not_allowed"));
        assert!(data["exit_codes"].as_array().expect("exits").iter().any(|c| c["code"] == 124));
    }
}
