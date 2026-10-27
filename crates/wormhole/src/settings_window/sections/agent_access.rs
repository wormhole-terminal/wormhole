//! Coding agents → Agent access: who may control Wormhole through its command line, MCP or socket, and
//! the programs the user has let in (`../repo-docs/docs/agent-permissions.md`). The mode is a row of its own;
//! each approved program is a row with the one action that fits, Revoke. Nothing is allowed here: an answer
//! to a question in Wormhole's window is the only way a program gets in, and taking one back is Revoke.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::access::{Capability, Kind};
use wormhole_core::view::sidebar::wording;

use super::agents::mark;
use crate::agent_access::{self, Approved, How, Which};
use crate::settings::UiColors;
use crate::settings_window::registry::{Segment, Setting};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

pub(super) const MODE: Setting = Setting::segmented(
    "agent-access",
    &[
        Segment::text(SETTINGS_AGENTS_ACCESS_OFF),
        Segment::text(SETTINGS_AGENTS_ACCESS_ASK),
        Segment::text(SETTINGS_AGENTS_ACCESS_OPEN),
    ],
    |s| match s.agent_access {
        AgentAccess::Off => 0,
        AgentAccess::Ask => 1,
        AgentAccess::Open => 2,
    },
    |s, ix| s.agent_access = [AgentAccess::Off, AgentAccess::Ask, AgentAccess::Open][ix.min(2)],
);

pub(super) const PROGRAMS: Setting = Setting::block("agent-access-programs", draw_programs);

/// The approved programs, or a line saying that nothing has asked.
fn draw_programs(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let rows = agent_access::approved(cx);
    if rows.is_empty() {
        return div()
            .py_3()
            .rule_top(ui)
            .text_body()
            .text_color(ui.muted)
            .child(text(SETTINGS_AGENTS_ACCESS_NONE))
            .into_any_element();
    }
    let now = agent_access::now_seconds();
    div()
        .flex()
        .flex_col()
        .children(rows.into_iter().enumerate().map(|(ix, row)| program_row(ix, row, now, cx)))
        .child(
            div()
                .py_2()
                .rule_top(ui)
                .text_caption()
                .text_color(ui.muted)
                .child(text(SETTINGS_AGENTS_ACCESS_CAPS_NOTE)),
        )
        .into_any_element()
}

/// What a program may do, one tick each. Closing is not among them: it always asks.
const SWITCHES: [Capability; 4] = [Capability::Read, Capability::Type, Capability::Run, Capability::Open];

fn capability_label(capability: Capability) -> &'static str {
    text(match capability {
        Capability::Read => SETTINGS_AGENTS_ACCESS_CAP_READ,
        Capability::Type => SETTINGS_AGENTS_ACCESS_CAP_TYPE,
        Capability::Run => SETTINGS_AGENTS_ACCESS_CAP_RUN,
        Capability::Open | Capability::Close => SETTINGS_AGENTS_ACCESS_CAP_OPEN,
    })
}

/// The ticks of one approved program: what it may do, changed at once and kept with the approval.
fn switches(
    ix: usize,
    which: Which,
    row: &Approved,
    ui: UiColors,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .gap_x_3()
        .pt_1()
        .children(SWITCHES.into_iter().map(|capability| {
            let this = cx.weak_entity();
            style::checkbox(
                SharedString::from(format!("agent-access-{ix}-{capability:?}")),
                ui,
                row.caps.contains(capability),
            )
            .in_form(capability_label(capability))
            .on_change(move |on, _, cx| {
                agent_access::set_capability(which, capability, on, cx);
                this.update(cx, |_, cx| cx.notify()).ok();
            })
        }))
        .into_any_element()
}

fn how_long(how: How) -> &'static str {
    text(match how {
        How::Session => SETTINGS_AGENTS_ACCESS_SESSION,
        How::UntilQuit => SETTINGS_AGENTS_ACCESS_UNTIL_QUIT,
        How::Always => SETTINGS_AGENTS_ACCESS_ALWAYS,
    })
}

/// A line under the name: how long it is allowed, who signed it, when it was last used.
fn detail(row: &Approved, now: i64) -> String {
    let mut parts = vec![how_long(row.how).to_string()];
    match row.kind {
        Kind::DeveloperId if !row.team.is_empty() => parts.push(agent_sheet_signed_team(&row.team)),
        Kind::DeveloperId | Kind::Apple => {}
        Kind::AdHoc => parts.push(text(SETTINGS_AGENTS_ACCESS_AD_HOC).to_string()),
        Kind::Unsigned => parts.push(text(SETTINGS_AGENTS_ACCESS_NOT_SIGNED).to_string()),
        Kind::Script | Kind::Scripts => {}
    }
    if row.how == How::Always {
        parts.push(match row.last_used {
            Some(at) => {
                let ago = wording::ago(std::time::Duration::from_secs(u64::try_from(now - at).unwrap_or(0)));
                settings_agents_access_last_used(&ago)
            }
            None => text(SETTINGS_AGENTS_ACCESS_NEVER_USED).to_string(),
        });
    }
    parts.join(" · ")
}

fn program_row(ix: usize, row: Approved, now: i64, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let (code_font, _) = crate::fonts::code_font(cx);
    let which = row.which;
    let tile = match row.agent {
        Some(id) => mark(ui, wormhole_core::agents::agent(id)),
        None => style::icon(gpui_kit::assets::IconName::Terminal).text_color(ui.muted).into_any_element(),
    };
    div()
        .flex()
        .items_center()
        .gap_3()
        .py_2()
        .rule_top(ui)
        .child(tile)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_body().font_weight(FontWeight::MEDIUM).truncate().child(row.label.clone()))
                .child(div().text_caption().text_color(ui.muted).truncate().child(detail(&row, now)))
                .when(!row.location.is_empty(), |column| {
                    column.child(
                        div()
                            .text_caption()
                            .text_color(ui.muted)
                            .font_family(code_font)
                            .truncate()
                            .child(row.location.clone()),
                    )
                })
                .child(switches(ix, which, &row, ui, cx)),
        )
        .child(
            Button::new(SharedString::from(format!("agent-access-revoke-{ix}")))
                .small()
                .outline()
                .label(text(SETTINGS_AGENTS_ACCESS_REVOKE))
                .on_click(cx.listener(move |_, _, _, cx| {
                    agent_access::revoke(which, cx);
                    cx.notify();
                })),
        )
        .into_any_element()
}
