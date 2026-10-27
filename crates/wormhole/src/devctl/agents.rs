//! `agents`, `agent-ask`, `agent-script` and `agent-grant`: what a test instance shows and does about
//! programs that ask to control it (`agent_access`). Nothing here is reachable from a request: they are
//! devctl's own, as `close-answer` is for the sheet before closing.

use anyhow::{Result, anyhow};
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::{CapSet, Identity};
use wormhole_core::storage::store::AgentGrantRow;

use super::Ctx;
use crate::agent_access::{self, Button, How, Script, Which};
use crate::app_state::{app, app_mut};
use crate::settings::settings;

/// `agents`: the mode, then each approved program (`approved <which> <label> <how> <kind>`), then each
/// question waiting (`asking <id> <label> <shown|waiting> <buttons>`). What a check reads of the page.
pub(super) fn agents(ctx: &mut Ctx, _: &str) -> Result<String> {
    let cx = &mut **ctx.cx;
    let on = settings(cx).feature_enabled(FeatureFlag::Automation);
    let mut lines = vec![format!(
        "automation {}\naccess {}",
        if on { "on" } else { "off" },
        settings(cx).agent_access.word()
    )];
    for row in agent_access::approved(cx) {
        let which = match row.which {
            Which::Memory(id) => format!("memory:{id}"),
            Which::Stored(id) => format!("stored:{id}"),
        };
        let how = match row.how {
            How::Session => "session",
            How::UntilQuit => "quit",
            How::Always => "always",
        };
        lines.push(format!("approved {which} {} {how} {}", row.label, row.kind.word()));
    }
    for question in agent_access::pending_questions(false, cx) {
        let buttons: Vec<&str> = question.buttons.iter().map(|button| word(*button)).collect();
        lines.push(format!(
            "asking {} {} {} {}",
            question.id,
            question.label,
            if question.shown { "shown" } else { "waiting" },
            buttons.join(",")
        ));
    }
    Ok(lines.join("\n"))
}

fn word(button: Button) -> &'static str {
    match button {
        Button::Deny => "deny",
        Button::Session => "session",
        Button::Quit => "quit",
        Button::Always => "always",
        Button::Keep => "keep",
        Button::DontKeep => "dont-keep",
    }
}

fn button_of(word: &str) -> Option<Button> {
    Some(match word.trim() {
        "deny" | "0" => Button::Deny,
        "session" | "once" | "1" => Button::Session,
        "quit" | "2" => Button::Quit,
        "always" | "3" => Button::Always,
        _ => return None,
    })
}

/// `agent-ask <deny|session|quit|always>`: answer the question that is up as that button does. A button the
/// question does not offer comes to the longest allowance it does.
pub(super) fn agent_ask(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let button = button_of(arg)
        .ok_or_else(|| anyhow!("agent-ask takes deny, session, quit or always, not {arg:?}"))?;
    if agent_access::answer_pending(button, ctx.cx) {
        Ok(String::new())
    } else {
        Err(anyhow!("no question is up"))
    }
}

/// `agent-script <hold|deny|session|quit|always>`: what answers questions from now on, as
/// `WORMHOLE_AGENT_ASK` does at start: `hold` leaves them waiting for `agent-ask`.
pub(super) fn agent_script(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let script = match arg.trim() {
        "hold" => Script::Hold,
        other => Script::Answer(
            button_of(other)
                .ok_or_else(|| anyhow!("agent-script takes hold, deny, session, quit or always"))?,
        ),
    };
    agent_access::set_script(script, ctx.cx);
    Ok(String::new())
}

/// `agent-grant scripts [quit|always]`: let Scripts in, as an answer to its question would, so that a
/// scenario can start approved. (A named agent is told apart by its signature, which a scenario cannot
/// know beforehand: its question is answered with `agent-ask`.)
pub(super) fn agent_grant(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let who = words.next().unwrap_or_default();
    if who != "scripts" {
        return Err(anyhow!("agent-grant takes scripts, then quit or always"));
    }
    let identity = Identity::scripts();
    match words.next().unwrap_or("quit") {
        "quit" => agent_access::grant_until_quit(identity, "Scripts", ctx.cx),
        "always" => {
            let profile = app(ctx.cx).active_profile().clone();
            let row = AgentGrantRow {
                key: identity.key(),
                label: "Scripts".into(),
                caps: CapSet::STANDARD.bits(),
                created: agent_access::now_seconds(),
                ..AgentGrantRow::default()
            };
            let store = app_mut(ctx.cx).store_of_mut(&profile).ok_or_else(|| anyhow!("no database"))?;
            store.save_agent_grant(&row)?;
        }
        other => return Err(anyhow!("agent-grant scripts takes quit or always, not {other:?}")),
    }
    Ok(String::new())
}
