//! The approval sheet: what a Probe asks for, in plain words, each permission with a switch of its own,
//! and the one confirmation that fits (a click, or Touch ID when anything left on is Touch ID level or
//! the Probe makes a dangerous combination). It stands over the settings window as the New profile
//! sheet does, and, like project trust, approves exactly what it showed: the request holds the bytes of
//! the folder as they were read for it, and the copy the Probe will run from is made from those.
//!
//! Cancel, a cancelled or failed Touch ID, and a write that fails change nothing.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::probe::approval::{self, Changes, Grants, Request};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::validate::{Confirmation, DeclaredPermission, ProgramClass};
use wormhole_core::storage::paths::DataRoot;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::platform::auth::{self, AuthError};
use crate::settings_window::{Section, SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

const SHEET_W: f32 = 520.0;
/// How much of the window's height the sheet's scrolling part may take; the buttons stay below it.
const SHEET_MAX_H: f32 = 0.72;

/// An open approval sheet.
pub(in crate::settings_window) struct Sheet {
    request: Request,
    grants: Grants,
    /// The Touch ID dialog is up.
    waiting: bool,
    error: Option<SharedString>,
    name: String,
}

impl Sheet {
    fn confirmation(&self) -> Confirmation {
        self.request.confirmation(&self.grants)
    }

    /// What the sheet shows.
    #[cfg(test)]
    pub(in crate::settings_window) fn request(&self) -> &Request {
        &self.request
    }

    /// The permissions left on, for a check to change.
    #[cfg(test)]
    pub(in crate::settings_window) fn grants_mut(&mut self) -> &mut Grants {
        &mut self.grants
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl SettingsWindow {
    /// Read the Probe `id` once, off the main thread, and show what it asks for. A Probe that cannot be
    /// approved (it changed into an invalid one since the list was read) says why instead.
    pub(in crate::settings_window) fn open_probe_sheet(&mut self, id: String, cx: &mut Context<Self>) {
        self.open_sheet_then(id, false, cx);
    }

    /// devctl: [`Self::open_probe_sheet`], and Approve pressed as soon as it is up.
    #[cfg(feature = "devctl")]
    pub(in crate::settings_window) fn devctl_approve_probe(&mut self, id: String, cx: &mut Context<Self>) {
        self.open_sheet_then(id, true, cx);
    }

    /// devctl's `state` line for the sheet that is open.
    #[cfg(feature = "devctl")]
    pub(in crate::settings_window) fn probe_sheet_line(&self) -> String {
        self.extensions.sheet.as_ref().map_or_else(String::new, |sheet| {
            let waiting = if sheet.waiting { "waiting" } else { "ready" };
            format!("probe-sheet {} {waiting}\n", sheet.request.id())
        })
    }

    fn open_sheet_then(&mut self, id: String, approve: bool, cx: &mut Context<Self>) {
        let root = DataRoot::current();
        self.extensions.work = Some(cx.spawn(async move |this, cx| {
            let asked = id.clone();
            let read = cx.background_executor().spawn(async move { approval::read_one(&root, &id) }).await;
            this.update(cx, |this, cx| {
                this.extensions.work = None;
                match read {
                    Ok(request) => {
                        let name = reveal(&request.folder().manifest.probe.name).into_owned();
                        let grants = request.initial_grants();
                        this.extensions.sheet =
                            Some(Sheet { request, grants, waiting: false, error: None, name });
                        if approve {
                            this.approve_probe(cx);
                        }
                    }
                    Err(err) => {
                        this.notice = Some((
                            Section::Extensions,
                            settings_extensions_cannot_approve(&reveal(&asked), &reveal(&err.to_string()))
                                .into(),
                        ));
                        this.scan_probes(cx);
                    }
                }
                cx.notify();
            })
            .ok();
        }));
    }

    pub(in crate::settings_window) fn close_probe_sheet(&mut self, cx: &mut Context<Self>) -> bool {
        let open = self.extensions.sheet.take().is_some();
        if open {
            cx.notify();
        }
        open
    }

    /// Approve was pressed: a click goes straight to the approval, anything above it asks the system
    /// first (Touch ID, or the login password), and the answer decides.
    pub(in crate::settings_window) fn approve_probe(&mut self, cx: &mut Context<Self>) {
        let Some(sheet) = self.extensions.sheet.as_mut() else { return };
        if sheet.waiting {
            return;
        }
        sheet.error = None;
        match sheet.confirmation() {
            Confirmation::Click => self.finish_approval(Ok(()), cx),
            Confirmation::TouchId => {
                sheet.waiting = true;
                let rx = auth::authenticate_rx(&settings_extensions_auth_reason(&sheet.name));
                self.extensions.work = Some(cx.spawn(async move |this, cx| {
                    let answer = rx.recv().await.unwrap_or(Err(AuthError::Cancelled));
                    this.update(cx, |this, cx| this.finish_approval(answer, cx)).ok();
                }));
            }
        }
        cx.notify();
    }

    /// The answer to the confirmation. Only a yes records anything; the sheet stays up for the rest, so
    /// the permissions can be looked at again.
    pub(in crate::settings_window) fn finish_approval(
        &mut self,
        answer: Result<(), AuthError>,
        cx: &mut Context<Self>,
    ) {
        let Some(sheet) = self.extensions.sheet.as_mut() else { return };
        sheet.waiting = false;
        match answer {
            Ok(()) => {}
            Err(AuthError::Cancelled) => {
                cx.notify();
                return;
            }
            Err(err) => {
                sheet.error = Some(settings_extensions_not_approved(&sheet.name, &err.to_string()).into());
                cx.notify();
                return;
            }
        }
        let (request, grants) = (sheet.request.clone(), sheet.grants.clone());
        let root = DataRoot::current();
        self.extensions.work = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { approval::approve(&root, &request, &grants, now()) })
                .await;
            this.update(cx, |this, cx| {
                this.extensions.work = None;
                match result {
                    Ok(()) => this.extensions.sheet = None,
                    Err(err) => {
                        if let Some(sheet) = this.extensions.sheet.as_mut() {
                            sheet.error =
                                Some(settings_extensions_not_approved(&sheet.name, &err.to_string()).into());
                        }
                    }
                }
                this.scan_probes(cx);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// The sheet over the window, with a scrim that keeps the page behind it still.
    pub(in crate::settings_window) fn render_probe_sheet(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let sheet = self.extensions.sheet.as_ref()?;
        let ui = ui(cx);
        let (code_font, _) = crate::fonts::code_font(cx);
        let request = &sheet.request;
        let probe = &request.folder().manifest.probe;
        let changes = request.changes();
        let again = changes.is_some();
        let title = if again {
            settings_extensions_sheet_title_again(&sheet.name)
        } else {
            settings_extensions_sheet_title(&sheet.name)
        };
        let summary = request.summary();

        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(style::panel_title(ui, title))
            .child(div().text_caption().text_color(ui.muted).child(settings_extensions_identity(
                &reveal(&probe.id),
                &reveal(&probe.version),
                &reveal(&probe.author),
            )))
            .child(div().text_body().child(reveal(&probe.description).into_owned()));
        if let Some(changes) = &changes {
            body = body.child(changes_block(ui, changes));
        }
        body = body.child(style::section_label(
            ui,
            if summary.permissions.is_empty() {
                text(SETTINGS_EXTENSIONS_ASKS_FOR_NOTHING)
            } else {
                text(SETTINGS_EXTENSIONS_ASKS_FOR)
            },
        ));
        let mut list = div().flex().flex_col();
        for (ix, permission) in summary.permissions.iter().enumerate() {
            list = list.child(permission_row(ui, &code_font, sheet, ix, permission, cx));
        }
        body = body.child(list);
        if !summary.combinations.is_empty() {
            body = body.child(style::section_label(ui, text(SETTINGS_EXTENSIONS_COMBINATIONS))).children(
                summary.combinations.iter().map(|combination| {
                    style::banner(
                        ui,
                        style::BannerKind::Danger,
                        false,
                        settings_extensions_combination(&reveal(&combination.description)),
                    )
                }),
            );
        }
        body = body.child(div().text_caption().text_color(ui.muted).child(
            settings_extensions_files_fingerprint(
                &request.file_count().to_string(),
                request.short_fingerprint(),
            ),
        ));

        let touch = sheet.confirmation() == Confirmation::TouchId;
        let approve_label = if sheet.waiting {
            text(SETTINGS_EXTENSIONS_WAITING)
        } else if touch {
            text(SETTINGS_EXTENSIONS_APPROVE_TOUCH_ID)
        } else {
            text(SETTINGS_EXTENSIONS_APPROVE)
        };
        let max_h = window.viewport_size().height * SHEET_MAX_H;
        let card = style::card(ui, ui.border_strong)
            .id("probe-sheet")
            .w(px(SHEET_W))
            .max_w_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .bg(ui.elevated)
            .occlude()
            .child(div().id("probe-sheet-body").max_h(max_h).overflow_y_scroll().child(body))
            .child(div().text_caption().text_color(ui.muted).child(if touch {
                text(SETTINGS_EXTENSIONS_CONFIRM_TOUCH_ID)
            } else {
                text(SETTINGS_EXTENSIONS_CONFIRM_CLICK)
            }))
            .children(
                sheet.error.clone().map(|error| div().text_caption().text_color(ui.danger_text).child(error)),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("probe-cancel")
                            .outline()
                            .small()
                            .label(text(SETTINGS_EXTENSIONS_CANCEL))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_probe_sheet(cx);
                            })),
                    )
                    .child(
                        Button::new("probe-approve")
                            .primary()
                            .small()
                            .label(approve_label)
                            .loading(sheet.waiting)
                            .on_click(cx.listener(|this, _, _, cx| this.approve_probe(cx))),
                    ),
            );
        Some(
            div()
                .id("probe-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(ui.shadow)
                .occlude()
                .on_click(cx.listener(|this, _, _, cx| {
                    // A click beside the sheet leaves it, but not while the system asks.
                    if !this.extensions.sheet.as_ref().is_some_and(|s| s.waiting) {
                        this.close_probe_sheet(cx);
                    }
                }))
                .child(card)
                .into_any_element(),
        )
    }
}

/// What differs from the approval the Probe is `Changed` against.
fn changes_block(ui: crate::settings::UiColors, changes: &Changes) -> Div {
    let mut block =
        div().flex().flex_col().gap_1().child(style::section_label(ui, text(SETTINGS_EXTENSIONS_SINCE)));
    if changes.files_only() {
        block = block.child(div().text_body().child(text(SETTINGS_EXTENSIONS_FILES_CHANGED)));
    }
    for added in &changes.added {
        block = block.child(
            div()
                .text_body()
                .text_color(ui.warning_text)
                .child(settings_extensions_now_asks(&reveal(&describe(added)))),
        );
    }
    for gone in &changes.removed {
        block = block.child(div().text_body().child(settings_extensions_no_longer_asks(&reveal(gone))));
    }
    block
}

/// A permission as a sentence, for the places that list it without a switch.
fn describe(permission: &DeclaredPermission) -> String {
    let detail = reveal(&permission.detail);
    match permission.key {
        "process" => settings_extensions_perm_process(&detail),
        "env" => settings_extensions_perm_env(&detail),
        "open" => settings_extensions_perm_open(&detail),
        "notify" => text(SETTINGS_EXTENSIONS_PERM_NOTIFY).to_string(),
        "agents" => settings_extensions_perm_agents(detail.strip_prefix("install-hooks:").unwrap_or(&detail)),
        "main" => text(SETTINGS_EXTENSIONS_PERM_CODE).to_string(),
        key => settings_extensions_perm_other(key, &detail),
    }
}

/// One permission: its switch, the sentence, the Touch ID tag, and for a program the command lines
/// that run it and what kind of program it is.
fn permission_row(
    ui: crate::settings::UiColors,
    code_font: &SharedString,
    sheet: &Sheet,
    ix: usize,
    permission: &DeclaredPermission,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let granted = sheet.grants.is_granted(permission);
    let toggled = permission.clone();
    let page = cx.weak_entity();
    let switch = style::toggle(("probe-grant", ix), granted, move |_, _, cx| {
        let permission = toggled.clone();
        page.update(cx, |page, cx| {
            if let Some(sheet) = page.extensions.sheet.as_mut() {
                sheet.grants.toggle(&permission);
            }
            cx.notify();
        })
        .ok();
    })
    .accessibility_label(describe(permission));
    let program = (permission.key == "process").then(|| program_lines(ui, code_font, sheet, permission));
    div()
        .flex()
        .flex_col()
        .gap_1()
        .py_2()
        .rule_top(ui)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().min_w_0().text_body().child(describe(permission)))
                .when(permission.confirmation == Confirmation::TouchId, |d| {
                    d.child(style::status_badge(ui, ui.muted, text(SETTINGS_EXTENSIONS_TOUCH_ID)))
                })
                .child(switch),
        )
        .children(program)
        .into_any_element()
}

/// Under a program: what kind it is, and each command line of the Probe that runs it, written out
/// (control, bidi and invisible characters shown as escapes).
fn program_lines(
    ui: crate::settings::UiColors,
    code_font: &SharedString,
    sheet: &Sheet,
    permission: &DeclaredPermission,
) -> Div {
    let program = permission.detail.as_str();
    let note = sheet.request.summary().programs.iter().find(|n| n.program == program).map(|note| match note
        .class
    {
        ProgramClass::RunsAnything => {
            Some((ui.danger_text, settings_extensions_runs_anything(&reveal(program))))
        }
        ProgramClass::RunsFolderCode => {
            Some((ui.danger_text, settings_extensions_runs_folder_code(&reveal(program))))
        }
        ProgramClass::ReadOnlyGit => Some((ui.muted, text(SETTINGS_EXTENSIONS_READ_ONLY_GIT).to_string())),
        ProgramClass::OwnScript => Some((ui.muted, text(SETTINGS_EXTENSIONS_OWN_SCRIPT).to_string())),
        ProgramClass::Other => None,
    });
    let manifest = &sheet.request.folder().manifest;
    let runs = manifest
        .contributes
        .commands
        .iter()
        .filter_map(|c| c.run.as_ref())
        .chain(manifest.contributes.status.iter().filter_map(|s| s.run.as_ref()))
        .filter(|argv| argv.first().is_some_and(|first| first == program))
        .map(|argv| argv_line(argv));
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(note.flatten().map(|(color, line)| div().text_caption().text_color(color).child(line)))
        .children(runs.map(|line| {
            div().flex().child(
                div()
                    .px_1()
                    .rounded(ui.radius_chip)
                    .bg(ui.hover)
                    .text_caption()
                    .font_family(code_font.clone())
                    .child(line),
            )
        }))
}

/// A command line for a person to read: each argument written out, and quoted when it has a space.
pub(super) fn argv_line(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            let shown = reveal(arg);
            if shown.is_empty() || shown.contains(char::is_whitespace) || shown.contains('"') {
                format!("\"{}\"", shown.replace('"', "\\\""))
            } else {
                shown.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::argv_line;

    #[test]
    fn a_command_line_is_written_so_that_its_arguments_can_be_told_apart() {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(argv_line(&argv(&["git", "log", "-1"])), "git log -1");
        assert_eq!(argv_line(&argv(&["say", "a b", ""])), "say \"a b\" \"\"");
        assert_eq!(argv_line(&argv(&["x", "a\"b"])), "x \"a\\\"b\"");
        assert!(
            argv_line(&argv(&["x", "\u{202e}evil"])).contains("\\u{202e}"),
            "bidi controls are written out"
        );
    }
}
