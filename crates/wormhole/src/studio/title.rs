//! The Studio's title row: the lights, "Theme Studio / <the theme's name> ▾" (the name opens the
//! theme menu), what it is based on, Unsaved; then Revert to Saved, Export… and the Save theme
//! split, whose menu has the other ways to write the theme out.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::ButtonGroup;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::popover::Popover;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens;

use super::document::Subject;
use super::document::history::StepLabel;
use super::names;
use super::save::Notice;
use super::{StudioWindow, ui};
use crate::settings::{SettingsStore, UiColors};
use crate::style::TitleDrag as _;
use crate::style::{self, Edges, TextRole};
use wormhole_text::keys::{
    STUDIO_APPEARANCE_DARK, STUDIO_APPEARANCE_LIGHT, STUDIO_COPY_JSON, STUDIO_EXPORT, STUDIO_EXPORT_JSON,
    STUDIO_SAVE_THEME, STUDIO_TITLE, STUDIO_UNSAVED, STUDIO_USE_IN_PROJECT, studio_title_based_on,
    studio_title_extends, studio_title_generated, studio_title_own,
};
use wormhole_text::text;

/// The name field while renaming.
const NAME_FIELD_W: f32 = 200.0;
/// The most room the theme's name takes in the title row: a longer one is cut with "…", before
/// anything else in the row gives way.
const NAME_MAX_W: f32 = 260.0;
/// The Save theme menu.
const MENU_W: f32 = 240.0;

/// One entry of the Save theme menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SaveAction {
    Save,
    /// ⇧⌘S: a copy in a file of its own, which the window edits from then on.
    Duplicate,
    Export,
    CopyJson,
    UseInProject,
}

impl SaveAction {
    /// The ways to write the theme out besides saving it: what to show, its key, and the
    /// separator before it.
    const MENU: [(SaveAction, wormhole_text::Key, Option<&'static str>, bool); 3] = [
        (SaveAction::Export, STUDIO_EXPORT_JSON, None, false),
        (SaveAction::CopyJson, STUDIO_COPY_JSON, None, false),
        (SaveAction::UseInProject, STUDIO_USE_IN_PROJECT, None, true),
    ];
}

impl StudioWindow {
    pub(super) fn render_title_bar(
        &self,
        background: Hsla,
        lights_end: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let tokens = &cx.global::<SettingsStore>().tokens;
        let appearance = if tokens.appearance == tokens::Appearance::Light {
            text(STUDIO_APPEARANCE_LIGHT)
        } else {
            text(STUDIO_APPEARANCE_DARK)
        };
        let base = super::document::places::base_under(&self.doc.file, self.doc.shown.as_deref());
        let extends = match (base, &self.doc.file.extends) {
            (Some(base), _) => {
                let from = wormhole_core::theme::tokens::slug::builtin_name(&base.from).unwrap_or(&base.from);
                match base.copied.as_deref() {
                    Some(day) => studio_title_based_on(from, &names::display_date(day), appearance),
                    // Made by Wormhole from the other appearance: it was not copied on a day.
                    None => studio_title_generated(from, appearance),
                }
            }
            (None, Some(base)) => studio_title_extends(base, appearance),
            (None, None) => studio_title_own(appearance),
        };
        let name: SharedString = self.doc.name().to_string().into();
        let title_name = if self.renaming {
            div().flex_none().w(px(NAME_FIELD_W)).child(Input::new(&self.name).small()).into_any_element()
        } else {
            // The name is the menu's trigger; its bounds are kept for the menu to hang from.
            let bounds = self.name_bounds.clone();
            div()
                .relative()
                .flex()
                .min_w_0()
                .max_w(px(NAME_MAX_W))
                .child(canvas(move |at, _, _| bounds.set(at), |_, _, _, _| {}).absolute().size_full())
                .child(
                    Button::new("studio-name")
                        .ghost()
                        .small()
                        .min_w_0()
                        .child(div().min_w_0().truncate().child(name))
                        .icon(IconName::ChevronDown)
                        .tooltip(wormhole_text::text(wormhole_text::keys::STUDIO_TOOLTIP_MENU))
                        .on_click(cx.listener(|this, _, window, cx| this.open_theme_menu(window, cx))),
                )
                .into_any_element()
        };
        div()
            .id("studio-titlebar")
            .flex_none()
            .h(px(style::TITLE_ROW_H))
            .flex()
            .items_center()
            .gap_2()
            // Room for the traffic lights.
            .pl(px(lights_end + style::SIDEBAR_INSET))
            .pr_3()
            .bg(background)
            .rule_bottom(ui)
            .title_drag()
            // The left side gives way (the caption, then the name, then the marks are cut off) and
            // the right side, with Save theme, never does.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .text_label()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(ui.muted)
                            .child(text(STUDIO_TITLE)),
                    )
                    .child(div().flex_none().text_color(ui.faint).child("/"))
                    .child(title_name)
                    .when(self.naming.is_some(), |d| {
                        d.child(
                            div()
                                .flex_none()
                                .text_caption()
                                .text_color(ui.faint)
                                .child(wormhole_text::text(wormhole_text::keys::STUDIO_NAME_PROMPT)),
                        )
                    })
                    .child(div().min_w_0().truncate().text_caption().text_color(ui.faint).child(extends))
                    .children(self.title_chips(ui, cx))
                    .when(self.dirty(), |d| {
                        d.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_caption()
                                .text_color(ui.warning_text)
                                .child(div().size(px(style::MARK_DOT)).rounded_full().bg(ui.warning))
                                .child(text(STUDIO_UNSAVED)),
                        )
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("studio-discard")
                            .ghost()
                            .small()
                            .label(wormhole_text::text(wormhole_text::keys::STUDIO_MENU_REVERT))
                            .disabled(!self.dirty())
                            .on_click(cx.listener(|this, _, window, cx| this.discard_changes(window, cx))),
                    )
                    .child(
                        Button::new("studio-export")
                            .outline()
                            .small()
                            .icon(IconName::Upload)
                            .label(text(STUDIO_EXPORT))
                            .on_click(cx.listener(|this, _, window, cx| this.export(window, cx))),
                    )
                    .child(self.save_split(ui, cx)),
            )
            .into_any_element()
    }

    /// Save theme, with the chevron that opens the other ways to save. The menu is anchored
    /// under the split and open while `save_menu` says so, so devctl can show it too.
    fn save_split(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let split = ButtonGroup::new("studio-save-split")
            .primary()
            .small()
            .child(
                Button::new("studio-save")
                    .label(text(STUDIO_SAVE_THEME))
                    .on_click(cx.listener(|this, _, window, cx| this.request_save(window, cx))),
            )
            .child(
                Button::new("studio-save-more")
                    .icon(IconName::ChevronDown)
                    .on_click(cx.listener(|this, _, _, cx| this.open_save_menu(cx))),
            );
        let this = cx.weak_entity();
        let on_open = this.clone();
        let project = self.menu_project;
        let menu = Popover::new("studio-save-menu")
            .anchor(Anchor::TopRight)
            .offset(px(4.0))
            .open(self.save_menu)
            .on_open_change(move |open, _, cx| {
                on_open
                    .update(cx, |studio, cx| {
                        studio.save_menu = *open;
                        cx.notify();
                    })
                    .ok();
            })
            // The menu hangs from the split's corner: an empty anchor stands in for it.
            .trigger(Button::new("studio-save-anchor").ghost().w(px(0.0)).h(px(0.0)));
        let menu =
            crate::select::menu_surface(menu, ui, px(MENU_W), ui.elevated).p_1().content(move |_, _, _| {
                let mut list = div().flex().flex_col();
                for (action, label, keys, separated) in SaveAction::MENU {
                    if separated {
                        list = list.child(div().my_1().h(ui.border_width).bg(ui.border));
                    }
                    let enabled = action != SaveAction::UseInProject || project;
                    let this = this.clone();
                    list = list.child(
                        style::row(SharedString::from(format!("studio-menu-{action:?}")), ui, false)
                            .when(!enabled, |d| d.text_color(ui.faint).cursor_default())
                            .child(div().flex_1().child(text(label)))
                            .children(
                                keys.map(|k| {
                                    div().text_caption().tabular_nums().text_color(ui.faint).child(k)
                                }),
                            )
                            .when(enabled, |d| {
                                d.on_click(move |_, window, cx| {
                                    this.update(cx, |studio, cx| studio.run_save_action(action, window, cx))
                                        .ok();
                                })
                            }),
                    );
                }
                list
            });
        div()
            .relative()
            .flex_none()
            .child(split)
            .child(div().absolute().right_0().bottom_0().child(menu))
            .into_any_element()
    }

    pub(super) fn open_save_menu(&mut self, cx: &mut Context<Self>) {
        self.menu_project = self.project_for_theme(cx).is_ok();
        self.save_menu = true;
        cx.notify();
    }

    pub(super) fn run_save_action(
        &mut self,
        action: SaveAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_menu = false;
        match action {
            SaveAction::Save => self.request_save(window, cx),
            SaveAction::Duplicate => self.duplicate(window, cx),
            SaveAction::Export => self.export(window, cx),
            SaveAction::CopyJson => self.copy_json(cx),
            SaveAction::UseInProject => self.use_in_project(cx),
        }
        cx.notify();
    }

    /// Click on the name (or Rename in the name-clash notice): edit it in place.
    pub(super) fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.renaming = true;
        let name = self.doc.name().to_string();
        self.name.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        cx.notify();
    }

    /// Return or leaving the field keeps the new name; an empty one keeps the old.
    pub(super) fn finish_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.renaming {
            return;
        }
        self.renaming = false;
        let name = self.name.read(cx).value().trim().to_string();
        if !name.is_empty()
            && self.doc.file.name.as_deref() != Some(name.as_str())
            && let Some(why) = self.rename_clash(&name)
        {
            // Two themes with one name would pick each other's file: the old name stays.
            self.notice = Some(Notice::Failed(why.into()));
            let old = self.doc.name().to_string();
            self.name.update(cx, |input, cx| input.set_value(old, window, cx));
        } else if !name.is_empty() && self.doc.file.name.as_deref() != Some(name.as_str()) {
            self.record_step(StepLabel::Rename, false, cx);
            self.doc.file.name = Some(name);
            self.publish_edit_titles(cx);
            // A new name clears a clash with the old one.
            self.notice = None;
        }
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    /// Why `name` can't be the name of the theme being edited: another theme (or a version of
    /// one) has it. Only a theme with a file of its own is checked here; a copy meets the same
    /// question when it is saved.
    fn rename_clash(&self, name: &str) -> Option<String> {
        let Subject::Own { path } = &self.doc.subject else { return None };
        if (self.themes.version_named)(name) {
            return Some(wormhole_text::keys::studio_name_is_a_version(name));
        }
        match (self.themes.saved_path)(name) {
            Some(other) if other != *path => Some(wormhole_text::keys::studio_name_taken(name)),
            _ => None,
        }
    }

    /// Return in the name field while the name is being asked for: keep the name and save.
    pub(super) fn commit_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let then_close = self.naming.take();
        self.finish_rename(window, cx);
        if let Some(then_close) = then_close {
            self.save_named(then_close, window, cx);
        }
    }

    pub(super) fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.naming = None;
        self.renaming = false;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
}
