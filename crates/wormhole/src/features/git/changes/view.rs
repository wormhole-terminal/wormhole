//! How the Changes tab draws: the status as sections of rows, the "compare with" chip and the refresh
//! button in the first section's head, or what stands in for the list (git is missing, the folder is no
//! repository, nothing changed).

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::{self, FileChange};
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::spoken;

use super::list::DiffTarget;
use super::rows::{ChangeRow, Group, file_name};
use super::{Changes, def::typed};
use crate::features::git::file_line::{Placement, change_row};
use crate::features::git::open_file::OpenTarget;
use crate::features::git::picker::{ChipOf, PickerChip};
use crate::features::git::problem::{self, GitTab};
use crate::features::git::submodule_row::{open_terminal_by_event, submodule_row};
use crate::settings::{SettingsStore, UiColors, ui as ui_main, ui_in};
use crate::style::{self, TextRole};
use crate::window::facts::WindowFacts;
use crate::window::feature::{FeatureEvent, RepoRequest};

/// The tab's content, built inline from `Workspace::render` on the feature's entity.
pub(super) fn tab(
    entity: &AnyEntity,
    facts: &WindowFacts<'_>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    typed(entity).update(cx, |changes, cx| changes.render_tab(facts, window, cx))
}

impl Changes {
    fn render_tab(&mut self, facts: &WindowFacts<'_>, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.follow(facts);
        let Some(active) = facts.active() else { return div().into_any_element() };
        let (session, terminal, git) = (active.id, active.terminal, active.git);
        // The rows are rebuilt for this frame's status; the entity is not notified, this draw is the redraw.
        self.sync_list(session, git.status.as_ref(), cx);
        let ui = ui_in(facts.profile, cx);
        let panel = div().flex().flex_col().flex_1().min_h_0();

        let Some(status) = &git.status else {
            if let Some(error) = git.error() {
                let this = cx.weak_entity();
                let retry = move |cx: &mut App| {
                    this.update(cx, |_, cx| cx.emit(FeatureEvent::Repo(RepoRequest::Refresh { session })))
                        .ok();
                };
                return panel
                    .child(style::panel_card(
                        ui,
                        problem::render_git_problem(facts.profile, GitTab::Changes, error, retry, cx),
                    ))
                    .into_any_element();
            }
            // Not a repository: nothing to compare or refresh.
            // "Reading" only until git has answered for this folder: a poll running on top of an
            // answer already shown must not swap the note for it and back.
            let answered = terminal.read(cx).cwd.as_deref().is_some_and(|c| git.answered_for(c));
            let text = if !answered {
                "Reading git status…"
            } else {
                "This terminal's folder isn't inside a git repository."
            };
            return panel.child(style::panel_card(ui, style::empty_note(ui, text))).into_any_element();
        };

        let Some(of) = self.of.get(&session) else { return div().into_any_element() };
        let Some(built) = of.list.built.as_ref() else { return div().into_any_element() };
        let view = Rc::new(ChangesView {
            session,
            files: built.files.clone(),
            target: built.target.clone(),
            open: OpenTarget::new(&status.root, cx),
            root: status.root.clone(),
            open_path: self.diff.as_ref().map(|d| d.file().path.clone()),
            width: f32::from(facts.panel_width),
            // The compare picker and refresh button sit in the first card's head, where the
            // panel's header row used to be.
            compare: self.picker.chip(
                cx.weak_entity(),
                ChipOf {
                    session,
                    terminal,
                    prefix: "vs",
                    branch: status.compare.as_deref(),
                    current: git.compare.clone(),
                },
                ui.branch,
            ),
            this: cx.weak_entity(),
        });

        let panel = panel
            .children(status.compare_missing.as_deref().map(|branch| problem::compare_missing(ui, branch)))
            .children(git.error().map(|error| problem::refresh_failed(ui, error)));
        if status.files.is_empty() {
            // The head stays, so the compare picker is still in reach.
            let head = view.render_band(Group::All, 0, false, true, ui, cx);
            return panel
                .child(style::panel_card(
                    ui,
                    div()
                        .flex()
                        .flex_col()
                        .child(head)
                        .child(style::empty_note(ui, "No changes. Working tree is clean.")),
                ))
                .into_any_element();
        }

        let in_list = view;
        let profile = facts.profile.clone();
        let list = &of.list;
        let ring = facts.panel_cursor.map(|(row, shown)| (row, shown.then_some(ui)));
        panel
            .child(list.rows.with_edges(
                list.rows.element_carded(ring, ui, ChangeRow::part, move |row, window, cx| {
                    in_list.render_row(row, ui_in(&profile, cx), window, cx)
                }),
                ui,
                "changes-scrollbar",
                cx,
            ))
            .into_any_element()
    }
}

/// What the Changes list's rows draw from, captured for the render callback.
struct ChangesView {
    session: i64,
    files: Rc<Vec<FileChange>>,
    /// What a click on a file opens its diff against.
    target: Rc<DiffTarget>,
    /// The repository, which a submodule's folder is under.
    root: PathBuf,
    /// The file whose diff is open: its row is the selected one.
    open_path: Option<String>,
    /// The icon that opens a file in the editor.
    open: OpenTarget,
    /// The panel's width, which file names are fitted to.
    width: f32,
    compare: PickerChip<Changes>,
    this: WeakEntity<Changes>,
}

impl ChangesView {
    /// One visible row of the Changes list.
    fn render_row(&self, row: &ChangeRow, ui: UiColors, window: &mut Window, cx: &App) -> AnyElement {
        let session = self.session;
        match row {
            // The first section's head carries the picker and refresh.
            ChangeRow::Band { group, count, folded, first } => {
                self.render_band(*group, *count, *folded, *first, ui, cx)
            }
            ChangeRow::FoldNote(text) => div()
                .px(px(style::CARD_INSET))
                .py_2()
                .text_caption()
                .text_color(ui.muted)
                .child(text.clone())
                .into_any_element(),
            ChangeRow::Dir { group, path, label, depth, collapsed } => {
                let (this, group, key) = (self.this.clone(), *group, path.clone());
                let (icons, tint) = (cx.global::<SettingsStore>().icons.as_deref(), ui_main(cx).muted);
                let icon = crate::icons::folder(icons, file_name(label), !collapsed, tint)
                    .unwrap_or_else(|| style::icon(IconName::Folder).into_any_element());
                style::folder_row(
                    SharedString::from(format!("dir-{}-{path}", group.tag())),
                    ui,
                    *depth,
                    icon,
                    label.clone(),
                    *collapsed,
                )
                .on_click(move |_, _, cx| {
                    this.update(cx, |this, cx| this.toggle_dir(session, group, key.clone(), cx)).ok();
                })
                .into_any_element()
            }
            ChangeRow::File { ix, depth, in_tree } => {
                let Some(file) = self.files.get(*ix) else { return div().into_any_element() };
                let (this, clicked) = (self.this.clone(), file.clone());
                let target = self.target.clone();
                let place = Placement {
                    in_tree: *in_tree,
                    depth: *depth,
                    selected: self.open_path.as_deref() == Some(file.path.as_str()),
                    width: self.width,
                };
                let row = match file.submodule {
                    Some(_) => submodule_row(
                        file,
                        place,
                        &self.root,
                        open_terminal_by_event(self.this.clone()),
                        ui,
                        cx,
                    ),
                    None => {
                        let row = change_row(file, place, ui, window, cx);
                        // A deleted file has nothing to open.
                        if file.kind == git::ChangeKind::Deleted {
                            row
                        } else {
                            self.open.file(&file.path).on_row(row, ui, &format!("change-{}", file.path))
                        }
                    }
                };
                row.on_click(move |_, _, cx| {
                    this.update(cx, |this, cx| this.open_change(session, clicked.clone(), &target, cx)).ok();
                })
                .into_any_element()
            }
            ChangeRow::Spacer => div().h(px(16.0)).into_any_element(),
        }
    }

    /// A section's card head: its title and file count, a click folds it. No +: nothing is added
    /// here. With `controls`, the compare picker and the refresh button come before the count.
    fn render_band(
        &self,
        group: Group,
        count: usize,
        folded: bool,
        controls: bool,
        ui: UiColors,
        cx: &App,
    ) -> AnyElement {
        let session = self.session;
        let this = self.this.clone();
        let refresh = self.this.clone();
        let trailing = if controls {
            vec![
                self.compare.render(ui, cx),
                style::icon_button("git-refresh", ui, IconName::RefreshCw, "Refresh")
                    .in_band()
                    .on_click(move |_, _, cx| {
                        refresh
                            .update(cx, |_, cx| cx.emit(FeatureEvent::Repo(RepoRequest::Refresh { session })))
                            .ok();
                    })
                    .into_any_element(),
            ]
        } else {
            Vec::new()
        };
        style::section_card(
            SharedString::from(format!("changes-band-{}", group.tag())),
            ui,
            style::BandParts {
                kind: style::BandKind::Section,
                section: style::Section::of(count, folded),
                title: group.title().into(),
                detail: None,
                count: (count > 0).then(|| count_grouped(count).into()),
                waiting: None,
                add: None,
                on_toggle: Some(Box::new(move |_, _, cx| {
                    this.update(cx, |this, cx| this.toggle_section(session, group, count, cx)).ok();
                })),
                drop_target: false,
                sticky: false,
                joined: false,
            },
            trailing,
        )
        .aria_label(spoken::section_band(group.title(), count, spoken::Counted::Files))
        .into_any_element()
    }
}
