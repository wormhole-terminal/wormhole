//! The Studio's title row: the lights, "Theme Studio / <the draft's name>"
//! (click the name to rename it), what it extends, Unsaved; then Discard, Export… and the
//! Save theme split, whose menu has the other ways to save.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::ButtonGroup;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::tokens;

use super::{StudioWindow, ui};
use crate::menubar::model::KeyEquivalent;
use crate::platform::{self, Item, Menu};
use crate::settings::SettingsStore;
use crate::style::TitleDrag as _;
use crate::style::{self, Edges, TextRole};

/// The name field while renaming.
const NAME_FIELD_W: f32 = 200.0;

/// One entry of the Save theme menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SaveAction {
    Save,
    SaveCopy,
    /// Save Copy, then the app uses the copy and the window edits it.
    SaveCopyAndSwitch,
    Export,
    CopyJson,
    UseInProject,
}

impl SaveAction {
    /// The menu, with Save as a copy as two entries: what to show, its
    /// key, and the separator before it.
    const MENU: [(SaveAction, &'static str, Option<bool>, bool); 6] = [
        (SaveAction::Save, "Save theme", Some(false), false),
        (SaveAction::SaveCopy, "Save Copy", Some(true), false),
        (SaveAction::SaveCopyAndSwitch, "Save Copy and Switch", None, false),
        (SaveAction::Export, "Export JSON…", None, true),
        (SaveAction::CopyJson, "Copy JSON", None, false),
        (SaveAction::UseInProject, "Use in this project…", None, true),
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
        let appearance = if tokens.appearance == tokens::Appearance::Light { "light" } else { "dark" };
        let extends = match &self.draft.extends {
            Some(base) => format!("extends {base} · {appearance}"),
            None => format!("its own palette · {appearance}"),
        };
        let name: SharedString = self.draft.name.clone().unwrap_or_default().into();
        let title_name = if self.renaming {
            div().flex_none().w(px(NAME_FIELD_W)).child(Input::new(&self.name).small()).into_any_element()
        } else {
            div()
                .id("studio-name")
                .flex_none()
                .flex()
                .items_center()
                .h(px(style::CONTROL_H))
                .px_2()
                .rounded(ui.radius)
                .text_body()
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(move |d| d.bg(ui.hover))
                .tooltip(|window, cx| Tooltip::new("Rename the theme").build(window, cx))
                .on_click(cx.listener(|this, _, window, cx| this.start_rename(window, cx)))
                .child(name)
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
            .child(
                div()
                    .flex_none()
                    .text_label()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ui.muted)
                    .child("Theme Studio"),
            )
            .child(div().flex_none().text_color(ui.faint).child("/"))
            .child(title_name)
            .child(div().min_w_0().truncate().text_caption().text_color(ui.faint).child(extends))
            .when(self.dirty, |d| {
                d.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_caption()
                        .text_color(ui.warning_text)
                        .child(div().size(px(style::MARK_DOT)).rounded_full().bg(ui.warning))
                        .child("Unsaved"),
                )
            })
            .child(div().flex_1())
            .child(
                Button::new("studio-discard")
                    .ghost()
                    .small()
                    .label("Discard")
                    .disabled(!self.dirty)
                    .on_click(cx.listener(|this, _, window, cx| this.discard_changes(window, cx))),
            )
            .child(
                Button::new("studio-export")
                    .outline()
                    .small()
                    .icon(IconName::Upload)
                    .label("Export…")
                    .on_click(cx.listener(|this, _, window, cx| this.export(window, cx))),
            )
            .child(self.save_split(cx))
            .into_any_element()
    }

    /// Save theme, with the system menu of the other ways to save.
    fn save_split(&self, cx: &mut Context<Self>) -> AnyElement {
        let anchor = platform::Anchor::default();
        let menu_anchor = anchor.clone();
        let split = ButtonGroup::new("studio-save-split")
            .primary()
            .small()
            .child(
                Button::new("studio-save")
                    .label("Save theme")
                    .on_click(cx.listener(|this, _, window, cx| this.request_save(window, cx))),
            )
            .child(Button::new("studio-save-more").icon(IconName::ChevronDown).on_click(cx.listener(
                move |this, _, window, cx| {
                    this.open_save_menu(menu_anchor.below(), window, cx);
                },
            )));
        div().relative().flex_none().child(anchor.probe()).child(split).into_any_element()
    }

    pub(super) fn open_save_menu(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let project = self.project_for_theme(cx).is_ok();
        let this = cx.weak_entity();
        let close = this.clone();
        let mut menu = Menu::new();
        for (action, label, keys, separated) in SaveAction::MENU {
            if separated {
                menu = menu.separator();
            }
            let this = this.clone();
            let key = keys.map(|shift| KeyEquivalent {
                key: "s".into(),
                command: true,
                shift,
                ..Default::default()
            });
            menu = menu.item(
                Item::new(label).key(key).enabled(action != SaveAction::UseInProject || project).on_select(
                    move |window, cx| {
                        this.update(cx, |studio, cx| studio.run_save_action(action, window, cx)).ok();
                    },
                ),
            );
        }
        let menu = menu.on_close(move |_, cx| {
            close
                .update(cx, |studio, cx| {
                    studio.save_menu = false;
                    cx.notify();
                })
                .ok();
        });
        self.save_menu = true;
        cx.notify();
        platform::show_menu(menu, at, window, cx);
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
            SaveAction::SaveCopy => self.save_copy(false, cx),
            SaveAction::SaveCopyAndSwitch => self.save_copy(true, cx),
            SaveAction::Export => self.export(window, cx),
            SaveAction::CopyJson => self.copy_json(cx),
            SaveAction::UseInProject => self.use_in_project(cx),
        }
        cx.notify();
    }

    /// Click on the name (or Rename in the name-clash notice): edit it in place.
    pub(super) fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.renaming = true;
        let name = self.draft.name.clone().unwrap_or_default();
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
        if !name.is_empty() && self.draft.name.as_deref() != Some(name.as_str()) {
            self.draft.name = Some(name);
            // A new name clears a clash with the old one.
            self.notice = None;
        }
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    pub(super) fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.renaming = false;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
}
