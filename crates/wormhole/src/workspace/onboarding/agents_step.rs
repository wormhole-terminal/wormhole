//! The last step: the coding agents found with their Install buttons, the project's trust,
//! the notifications switch, or the primer when no agent is on PATH.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use wormhole_core::agents::integrations::State;
use wormhole_core::agents::{AGENTS, Agent};
use wormhole_core::keymap::{self, ActionId};
use wormhole_core::onboarding::Event;

use super::Intro;
use crate::settings::{UiColors, settings};
use crate::style::{self, Edges, TextRole};
use crate::workspace::app::app;
use crate::workspace::settings_page::update_settings;
use crate::workspace::{Workspace, project};

impl Workspace {
    /// Step 3: each agent on PATH with its own Install, the notifications switch, and the folder's
    /// trust when it has a `.wormhole/` folder that isn't trusted.
    pub(super) fn render_agents_step(
        &self,
        sheet: &Intro,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (code_font, _) = crate::fonts::code_font(cx);
        let Some(agents) = &sheet.agents else {
            return div()
                .px_6()
                .py_3()
                .text_body()
                .text_color(ui.muted)
                .child("Looking for agents…")
                .into_any_element();
        };
        let mut list = div().flex().flex_col().rule_top(ui);
        for found in &agents.found {
            let agent = found.agent;
            let (detail, action): (SharedString, AnyElement) = match (found.spec, found.state) {
                (Some(_), Some(State::Installed)) => (
                    "Hooks in".into(),
                    style::status_badge(ui, ui.success_text, "Installed")
                        .child(style::icon(IconName::Check))
                        .into_any_element(),
                ),
                (Some(_), state) => {
                    let id = agent.id;
                    let label = if state == Some(State::Partial) { "Repair" } else { "Install" };
                    (
                        "On PATH ·".into(),
                        Button::new(SharedString::from(format!("onboarding-install-{id:?}")))
                            .outline()
                            .small()
                            .label(label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.onboarding_event(Event::Install(id), window, cx)
                            }))
                            .into_any_element(),
                    )
                }
                (None, _) => (
                    "On PATH · status from its output".into(),
                    style::status_badge(ui, ui.muted, "Reading output").into_any_element(),
                ),
            };
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_2()
                    .rule_bottom(ui)
                    .child(monogram(ui, agent, false))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_body().font_weight(FontWeight::MEDIUM).child(agent.name))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .min_w_0()
                                    .text_caption()
                                    .text_color(ui.muted)
                                    .child(detail)
                                    .children(found.path.clone().filter(|_| found.spec.is_some()).map(
                                        |path| {
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .px_1()
                                                .rounded(ui.radius_chip)
                                                .bg(ui.hover)
                                                .font_family(code_font.clone())
                                                .child(path)
                                        },
                                    )),
                            ),
                    )
                    .child(action),
            );
        }
        if let Some(first) = agents.others.first() {
            let more = agents.others.len() - 1;
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_2()
                    .rule_bottom(ui)
                    .child(monogram(ui, first, true))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_body().text_color(ui.faint).child(first.name))
                            .child(
                                div()
                                    .text_caption()
                                    .text_color(ui.faint)
                                    .child(format!("Not found on PATH · {more} more recognised")),
                            ),
                    ),
            );
        }
        let notifications = settings(cx).notifications;
        div()
            .flex()
            .flex_col()
            .child(list)
            .children(sheet.problem.clone().map(|problem| {
                div().px_6().pt_3().child(style::banner(ui, style::BannerKind::Danger, false, problem))
            }))
            .child(div().px_6().py_3().text_caption().text_color(ui.muted).child(
                "Start installs nothing; only Install does. Hooks add entries of their own and back up what \
                 they change. Skip any agent: Settings › Coding agents adds or removes hooks later.",
            ))
            .child(switch_row(
                "onboarding-notifications",
                ui,
                "Notify me when a background terminal needs me or finishes",
                notifications,
                |on, _, cx| update_settings(cx, |s| s.notifications = on),
            ))
            .children(self.render_sheet_trust(sheet, ui, cx))
            .into_any_element()
    }

    /// The folder's `.wormhole/`: what works now, what waits for trust, and the one button
    /// that asks for it (Touch ID), or Later.
    pub(super) fn render_sheet_trust(
        &self,
        sheet: &Intro,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if sheet.trust_later {
            return None;
        }
        let root = match sheet.model.folder() {
            Some(folder) => wormhole_core::project::find_project(folder).map(|f| f.root),
            None => self.active_session().project.clone(),
        }?;
        let project = project::load_project_info(&root, cx)?;
        if project.status == wormhole_core::project::TrustStatus::Trusted {
            return None;
        }
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = wormhole_core::project::dir_name_at(&root);
        let commands = project.request.as_ref().map_or(0, |r| r.commands.len());
        let waits = match commands {
            0 => "Nothing in it runs commands yet.".to_string(),
            1 => "Its action runs a command, so it waits until you trust the folder.".to_string(),
            n => format!("Its actions run {n} commands, so they wait until you trust the folder."),
        };
        let approving = app(cx).hub.approving;
        let trust_root = root.clone();
        Some(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .px_6()
                .py_3()
                .rule_top(ui)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(style::icon(IconName::Shield).text_color(ui.muted))
                        .child(
                            div()
                                .text_body()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{name}/ has a {dir}/ folder")),
                        ),
                )
                .child(
                    div()
                        .text_caption()
                        .text_color(ui.muted)
                        .child(format!("Its theme and notes work now. {waits}")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new("onboarding-trust")
                                .outline()
                                .small()
                                .icon(IconName::FingerprintPattern)
                                .label(if approving {
                                    "Waiting for approval…"
                                } else {
                                    "Trust with Touch ID…"
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.onboarding_event(Event::Trust(trust_root.clone()), window, cx)
                                })),
                        )
                        .child(
                            Button::new("onboarding-trust-later").ghost().small().label("Later").on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(sheet) = &mut this.first_run.intro {
                                        sheet.trust_later = true;
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                )
                .into_any_element(),
        )
    }

    /// The primer: Wormhole as a terminal, the keys worth knowing, and what changes once an
    /// agent is installed.
    pub(super) fn render_primer_step(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        // The list; the keys come from the key table, so they stay what the menus say (the
        // sheet's is ⌥⌘/ since ⌘/ became the status bar's).
        const KEYS: [(&str, &[ActionId]); 5] = [
            ("Jump to anything", &[ActionId::CommandPalette]),
            ("See every shortcut", &[ActionId::KeyboardShortcuts]),
            ("Show what changed in git", &[ActionId::ShowChanges]),
            ("Jot a note for this folder", &[ActionId::NewNote]),
            ("Hide the side panels", &[ActionId::ToggleSidebar, ActionId::ToggleRightPanel]),
        ];
        let notes_on = crate::workspace::notes::notes_on(cx);
        let command_done = settings(cx).notify_command_done;
        div()
            .flex()
            .flex_col()
            .child(
                style::flush_row("onboarding-terminal", ui, true)
                    .py_2()
                    .items_start()
                    .cursor_default()
                    .child(style::lead(style::icon(IconName::SquareTerminal).text_color(ui.selected_text)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_body()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Use Wormhole as a terminal"),
                            )
                            .child(div().text_caption().text_color(ui.selected_muted).child(if notes_on {
                                "Notes, next steps and the git panel work in any shell."
                            } else {
                                "The git panel works in any shell."
                            })),
                    ),
            )
            .child(div().px_6().pt_4().pb_2().child(style::section_label(ui, "Things to try first")))
            .children(KEYS.into_iter().filter(|(_, a)| notes_on || a != &[ActionId::NewNote]).map(
                |(label, actions)| {
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(style::ROW_H + 4.0))
                        .px_6()
                        .rule_top(ui)
                        .child(div().text_body().child(label))
                        .child(div().flex().gap_1().children(
                            actions.iter().map(|action| style::keycap(ui, keymap::glyphs(*action))),
                        ))
                },
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .rule_top(ui)
                    .text_caption()
                    .text_color(ui.muted)
                    .child(style::icon(IconName::Info))
                    .child(div().flex_1().min_w_0().child(format!(
                        "No coding agent found. Install Claude Code, Codex or one of {} others later and \
                         Wormhole recognises it in any terminal, then offers its hooks once.",
                        AGENTS.len().saturating_sub(2)
                    ))),
            )
            .child(switch_row(
                "onboarding-command-done",
                ui,
                "Notify me when a long command in a background terminal finishes",
                command_done,
                |on, _, cx| update_settings(cx, |s| s.notify_command_done = on),
            ))
            .into_any_element()
    }
}

/// A row with a bell, a line of text and a switch (the notifications on the last step).
fn switch_row(
    id: &'static str,
    ui: UiColors,
    label: &'static str,
    on: bool,
    set: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_6()
        .py_3()
        .rule_top(ui)
        .child(style::icon(IconName::Bell).text_color(ui.muted))
        .child(div().flex_1().min_w_0().text_body().child(label))
        .child(style::toggle(id, on, set))
        .into_any_element()
}

/// An agent's mark (CC, CX, GM…) in a square.
fn monogram(ui: UiColors, agent: &Agent, faint: bool) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(style::CONTROL_H))
        .rounded(ui.radius)
        .bg(ui.hover)
        .text_caption()
        .font_weight(FontWeight::BOLD)
        .text_color(if faint { ui.faint } else { ui.text })
        .child(agent.monogram)
        .into_any_element()
}
