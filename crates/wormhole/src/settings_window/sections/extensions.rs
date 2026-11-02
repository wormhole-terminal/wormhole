//! Extensions: the Probes in the data folder, one row each with its state and a switch
//! (`repo-docs/docs/probes.md`). Switching one on opens the approval sheet (`extensions_sheet`); off
//! needs no question. "On" is a stored approval that later steps read: nothing here runs a Probe.
//!
//! The folder is read on the background executor when the page opens, when the window comes forward
//! while the page is showing and from the Refresh button, never on a timer and never in a frame.
//! A Probe's words are written by whoever made the folder, so every one is passed through
//! `probe::hidden::reveal` before it is drawn.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::ipc::control::COMMAND_NAME;
use wormhole_core::probe::approval::{self, Entry, KeyProblem, ProbeState};
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::scan::MAX_ENTRIES;
use wormhole_core::storage::paths::DataRoot;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::extensions_sheet::Sheet;
use super::extensions_text::verify_text;
use crate::settings::UiColors;
use crate::settings_window::registry::Setting;
use crate::settings_window::{Section, SettingsWindow, ui};
use crate::style::{self, TextRole};

pub(super) const SETTINGS: &[Setting] = &[Setting::block("probes", draw_probes)];

/// What the folder held when it was last read.
pub(in crate::settings_window) enum Listing {
    Probes {
        entries: Vec<Entry>,
        /// Why the keys approvals are checked with could not be read: every Probe is off, and the page says so.
        key_problem: Option<KeyProblem>,
    },
    /// More entries than a list of Probes can be: none are listed.
    TooMany,
}

/// The Extensions page's state in the window.
#[derive(Default)]
pub(in crate::settings_window) struct Extensions {
    /// `None` until the first read finishes.
    pub(in crate::settings_window) listing: Option<Listing>,
    /// The read in flight; a newer one replaces (cancels) it.
    scan: Option<Task<()>>,
    /// The approval sheet over the window.
    pub(in crate::settings_window) sheet: Option<Sheet>,
    /// Reading a Probe for its sheet, turning one off or recording an approval.
    pub(super) work: Option<Task<()>>,
}

impl Extensions {
    /// Whether the row of `folder` offers to forget its approval.
    #[cfg(test)]
    pub(in crate::settings_window) fn can_forget(&self, folder: &str) -> Option<bool> {
        match self.listing.as_ref()? {
            Listing::Probes { entries, .. } => {
                entries.iter().find(|e| e.folder == folder).map(|e| e.can_forget())
            }
            Listing::TooMany => None,
        }
    }

    /// How many folders the last read listed; `None` before it finished or when there were too many.
    #[cfg(test)]
    pub(in crate::settings_window) fn listed(&self) -> Option<usize> {
        match self.listing.as_ref()? {
            Listing::Probes { entries, .. } => Some(entries.len()),
            Listing::TooMany => None,
        }
    }

    /// Whether the last read could not read the approvals' key.
    #[cfg(test)]
    pub(in crate::settings_window) fn key_unavailable(&self) -> bool {
        matches!(self.listing, Some(Listing::Probes { key_problem: Some(_), .. }))
    }

    /// Whether the row of `folder` says its stored approval is not this Wormhole's.
    #[cfg(test)]
    pub(in crate::settings_window) fn unsealed(&self, folder: &str) -> Option<bool> {
        match self.listing.as_ref()? {
            Listing::Probes { entries, .. } => {
                entries.iter().find(|e| e.folder == folder).map(|e| e.unsealed)
            }
            Listing::TooMany => None,
        }
    }

    /// The state of the Probe in `folder`, as the last read found it.
    #[cfg(test)]
    pub(in crate::settings_window) fn state_of(&self, folder: &str) -> Option<&ProbeState> {
        match self.listing.as_ref()? {
            Listing::Probes { entries, .. } => entries.iter().find(|e| e.folder == folder).map(|e| &e.state),
            Listing::TooMany => None,
        }
    }
}

/// The folder read and tidied: approvals and copies of Probes that are gone are forgotten, but only
/// after a read that saw the whole folder.
fn read_listing(root: &DataRoot) -> Listing {
    match approval::list(root) {
        Ok(listing) => {
            // Approvals of Probes that are gone are forgotten only after a read that saw the whole folder.
            if let Err(err) = approval::sweep(root, listing.present().as_ref()) {
                tracing::warn!("tidying the approved Probes: {err}");
            }
            Listing::Probes { entries: listing.entries, key_problem: listing.key_problem }
        }
        Err(_) => Listing::TooMany,
    }
}

impl SettingsWindow {
    /// Read the Probes folder again in the background; what is shown stays until the new list arrives.
    pub(in crate::settings_window) fn scan_probes(&mut self, cx: &mut Context<Self>) {
        let root = DataRoot::current();
        self.extensions.scan = Some(cx.spawn(async move |this, cx| {
            let listing = cx.background_executor().spawn(async move { read_listing(&root) }).await;
            this.update(cx, |this, cx| {
                this.extensions.listing = Some(listing);
                this.extensions.scan = None;
                cx.notify();
            })
            .ok();
        }));
    }

    /// Turn the Probe `id` off: its approval and its copy go, with no question.
    pub(in crate::settings_window) fn turn_probe_off(
        &mut self,
        id: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let root = DataRoot::current();
        self.extensions.work = Some(cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { approval::turn_off(&root, &id) }).await;
            this.update(cx, |this, cx| {
                this.notice = result.err().map(|err| {
                    (
                        Section::Extensions,
                        settings_extensions_not_turned_off(&reveal(&name), &err.to_string()).into(),
                    )
                });
                this.extensions.work = None;
                this.scan_probes(cx);
                cx.notify();
            })
            .ok();
        }));
    }

    /// The switch of a row was flipped.
    fn probe_switched(&mut self, id: String, name: String, on: bool, cx: &mut Context<Self>) {
        if on {
            self.open_probe_sheet(id, cx);
        } else {
            self.turn_probe_off(id, name, cx);
        }
    }
}

fn state_chip(state: &ProbeState) -> style::ChipKind {
    match state {
        ProbeState::On => style::ChipKind::Good,
        ProbeState::Changed => style::ChipKind::Warn,
        ProbeState::Invalid(_) => style::ChipKind::Danger,
        ProbeState::Off | ProbeState::Unsupported => style::ChipKind::Quiet,
    }
}

fn state_word(state: &ProbeState) -> &'static str {
    match state {
        ProbeState::Off => text(SETTINGS_EXTENSIONS_STATE_OFF),
        ProbeState::On => text(SETTINGS_EXTENSIONS_STATE_ON),
        ProbeState::Changed => text(SETTINGS_EXTENSIONS_STATE_CHANGED),
        ProbeState::Invalid(_) => text(SETTINGS_EXTENSIONS_STATE_INVALID),
        ProbeState::Unsupported => text(SETTINGS_EXTENSIONS_STATE_UNSUPPORTED),
    }
}

/// The page: the list, or why there is none yet.
fn draw_probes(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    match &window.extensions.listing {
        None => div()
            .py_3()
            .text_body()
            .text_color(ui.muted)
            .child(text(SETTINGS_EXTENSIONS_LOOKING))
            .into_any_element(),
        Some(Listing::TooMany) => style::notice(
            ui,
            style::NoticeKind::Warning,
            settings_extensions_too_many(&MAX_ENTRIES.to_string()),
        )
        .into_any_element(),
        Some(Listing::Probes { entries, key_problem }) => {
            let rows = if entries.is_empty() {
                empty(ui, cx)
            } else {
                div()
                    .flex()
                    .flex_col()
                    .children(entries.iter().map(|entry| probe_row(ui, entry, cx)))
                    .into_any_element()
            };
            let Some(problem) = key_problem else { return rows };
            let notice = match problem {
                KeyProblem::Unreadable => text(SETTINGS_EXTENSIONS_KEY_UNAVAILABLE),
                KeyProblem::InRealDataFolder => text(SETTINGS_EXTENSIONS_DEV_DATA_FOLDER),
            };
            // Over the rows, whatever they are: every one of them reads as off while the key can't be read.
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::notice(ui, style::NoticeKind::Warning, notice.to_string()))
                .child(rows)
                .into_any_element()
        }
    }
}

/// Where Probes live and how to make one.
fn empty(ui: UiColors, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let folder = DataRoot::current().probes_dir();
    let shown = crate::settings_window::with_tilde(&folder);
    let to_open = folder.clone();
    let command = format!("{COMMAND_NAME} probe new"); // text: a command line, not interface text
    style::empty_list(
        ui,
        style::EmptyList {
            icon: IconName::Puzzle,
            title: text(SETTINGS_EXTENSIONS_NONE_TITLE).into(),
            hint: format!(
                "{} {}",
                settings_extensions_none_where(&shown),
                settings_extensions_none_new(&command)
            )
            .into(),
            actions: vec![
                style::form_button("probes-show-folder")
                    .outline()
                    .icon(IconName::FolderOpen)
                    .label(text(SETTINGS_EXTENSIONS_SHOW_FOLDER))
                    .on_click(
                        cx.listener(move |_, _, _, cx| crate::settings_window::open_folder(&to_open, cx)),
                    )
                    .into_any_element(),
            ],
        },
    )
    .into_any_element()
}

/// One Probe: its mark, name and state, who and which version, what it says it is, and the switch.
fn probe_row(ui: UiColors, entry: &Entry, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let folder = entry.folder.clone();
    let id = SharedString::from(format!("probe-{folder}"));
    let (title, identity, description) = match &entry.probe {
        Some(probe) => (
            reveal(&probe.name).into_owned(),
            settings_extensions_identity(&reveal(&probe.id), &reveal(&probe.version), &reveal(&probe.author)),
            reveal(&probe.description).into_owned(),
        ),
        None => (reveal(&folder).into_owned(), String::new(), String::new()),
    };
    let state = &entry.state;
    // An unsealed approval is a stored one that reads as none: off, but worth a warning's colour.
    let chip_kind = if entry.unsealed { style::ChipKind::Warn } else { state_chip(state) };
    let word = if entry.unsealed { text(SETTINGS_EXTENSIONS_STATE_UNSEALED) } else { state_word(state) };
    let can_switch = state.can_approve() || state.is_on();
    let probe_id = entry.probe.as_ref().map_or_else(|| folder.clone(), |probe| probe.id.clone());
    let switch_name = title.clone();
    let switch = style::toggle(SharedString::from(format!("{id}-switch")), state.is_on(), {
        let this = cx.weak_entity();
        move |on, _, cx| {
            let (probe_id, name) = (probe_id.clone(), switch_name.clone());
            this.update(cx, |this, cx| this.probe_switched(probe_id, name, on, cx)).ok();
        }
    })
    .disabled(!can_switch)
    .accessibility_label(title.clone());

    let again = (*state == ProbeState::Changed).then(|| {
        let (probe_id, name) = (entry.folder.clone(), title.clone());
        style::form_button(SharedString::from(format!("{id}-again")))
            .outline()
            .label(text(SETTINGS_EXTENSIONS_APPROVE_AGAIN))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.probe_switched(probe_id.clone(), name.clone(), true, cx)
            }))
    });
    let forget = entry.can_forget().then(|| {
        let (probe_id, name) = (entry.folder.clone(), title.clone());
        style::form_button(SharedString::from(format!("{id}-forget")))
            .ghost()
            .label(text(SETTINGS_EXTENSIONS_FORGET))
            .on_click(
                cx.listener(move |this, _, _, cx| this.turn_probe_off(probe_id.clone(), name.clone(), cx)),
            )
    });
    let note = match state {
        ProbeState::Changed => Some(
            div()
                .child(match &entry.copy_problem {
                    Some(problem) => verify_text(problem),
                    None => text(SETTINGS_EXTENSIONS_CHANGED_NOTE).to_string(),
                })
                .into_any_element(),
        ),
        ProbeState::Unsupported => {
            Some(div().child(text(SETTINGS_EXTENSIONS_UNSUPPORTED_NOTE)).into_any_element())
        }
        ProbeState::Off if entry.unsealed => {
            Some(div().child(text(SETTINGS_EXTENSIONS_UNSEALED_NOTE)).into_any_element())
        }
        _ => None,
    };
    let invalid = match state {
        ProbeState::Invalid(reason) => Some(style::notice(
            ui,
            style::NoticeKind::Danger,
            settings_extensions_invalid_reason(&reveal(reason)),
        )),
        _ => None,
    };
    let mut parts = style::TileRowParts::new(
        style::RowTile::Icon {
            fill: crate::settings_window::nav::page_tile(Section::Extensions, cx),
            icon: IconName::Puzzle,
        },
        title,
    );
    parts.chip = Some((chip_kind, word.into()));
    parts.hint = Some(identity).filter(|identity| !identity.is_empty()).map(Into::into);
    parts.more = Some(description).filter(|description| !description.is_empty()).map(Into::into);
    parts.controls = [forget.map(|b| b.into_any_element()), again.map(|b| b.into_any_element())]
        .into_iter()
        .flatten()
        .chain([switch.into_any_element()])
        .collect();
    div()
        .flex()
        .flex_col()
        .child(style::tile_row(id, ui, parts))
        .children(note.map(|note| {
            div().pb_2().pl(px(style::ROW_TILE + 12.0)).text_caption().text_color(ui.muted).child(note)
        }))
        .children(invalid.map(|banner| div().pb_2().pl(px(style::ROW_TILE + 12.0)).child(banner)))
        .into_any_element()
}

/// The Refresh button, at the right of the page's title.
pub(in crate::settings_window) fn header_action(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    style::icon_button("probes-refresh", ui, IconName::RefreshCw, text(SETTINGS_EXTENSIONS_REFRESH))
        .on_click(cx.listener(|this, _, _, cx| {
            this.scan_probes(cx);
            cx.notify();
        }))
        .into_any_element()
}
