//! The component sheet's card sections: rows with their keys and reset arrows, rows with a tile, hints,
//! checkboxes, buttons, the row that opens more and the shortcut recorder, each with the board's own
//! samples.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::Cards;
use crate::ui::{
    CheckboxControlProps, Force, MenuFaceProps, MenuItemProps, MoreRowProps, SettingsRowProps, StepperProps,
    TileRowProps,
};

impl Cards<'_> {
    pub(super) fn checkboxes(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let boxed = |id: &'static str, on| {
            CheckboxControlProps { id: id.into(), label: id.into(), on, on_click: None }
                .render(&look)
                .into_any_element()
        };
        let both = div()
            .flex()
            .gap(px(16.0))
            .mt(px(6.0))
            .child(div().flex().items_center().gap(px(6.0)).child(boxed("cb-s", true)).child("Settings"))
            .child(
                div().flex().items_center().gap(px(6.0)).child(boxed("cb-t", false)).child("Theme Studio"),
            );
        let translucent = SettingsRowProps {
            label: div().flex().flex_col().child("Also translucent").child(both).into_any_element(),
            ..self.plain_row("cb-tr", "Also translucent", div().into_any_element())
        }
        .render(&look)
        .into_any_element();
        let card = self
            .card()
            .child(self.row("cb-on", "Ticked", Force::None, boxed("cb-on-c", true)))
            .child(self.row("cb-off", "Not ticked", Force::None, boxed("cb-off-c", false)))
            .child(translucent);
        self.section(
            "Checkbox",
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(card)
                .child(self.caption("Agent access, Also translucent, and Import’s review.")),
        )
    }

    pub(super) fn buttons(&self) -> Vec<AnyElement> {
        let card = self
            .card()
            .p(px(14.0))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.0))
            .child(self.button("b1", "Show in Finder", false, false, false, Force::None))
            .child(self.button("b2", "Install Update", true, false, false, Force::None))
            .child(self.button("b3", "Reset…", false, true, false, Force::None))
            .child(self.button("b4", "Make Default", false, false, true, Force::None))
            .child(div().text_size(px(12.0)).text_color(self.t("primary")).child("Edit config.json"));
        self.section("Buttons", card)
    }

    pub(super) fn more_rows(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let more = MoreRowProps {
            id: "sheet-more".into(),
            label: "Approved programs".into(),
            value: Some("None".into()),
            open: false,
            closed: true,
            on_click: None,
        }
        .render(&look);
        let field = div()
            .w(px(180.0))
            .h(px(24.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .rounded(px(6.0))
            .border_1()
            .border_color(self.t("primary"))
            .bg(self.t("input.background"))
            .child("Default")
            .into_any_element();
        let version = div().text_color(self.t("text.muted")).child("1.5.0").into_any_element();
        let card = self
            .card()
            .child(more)
            .child(self.row("sheet-name", "Name", Force::None, field))
            .child(self.row("sheet-version", "Version", Force::None, version));
        // The font size stepper has no row of its own on the board; it belongs to the font picker.
        let _ = StepperProps {
            id: "sheet-stepper".into(),
            label: "Font size".into(),
            smaller: "Smaller".into(),
            larger: "Larger".into(),
            value: "14 pt".into(),
            on_smaller: None,
            on_larger: None,
        };
        self.section("Row that opens more, text field, value", card)
    }

    pub(super) fn recorder(&self, cx: &App) -> Vec<AnyElement> {
        use crate::style::{RecorderState, shortcut_recorder};
        let ui = crate::settings::ui(cx);
        let state = |id: &'static str, state| shortcut_recorder(id, ui, state, None).into_any_element();
        let card = self
            .card()
            .child(self.row("rc-1", "Not set", Force::None, state("rc-c1", RecorderState::Empty)))
            .child(self.row(
                "rc-2",
                "Recording",
                Force::None,
                state("rc-c2", RecorderState::Recording(Vec::new())),
            ))
            .child(self.row(
                "rc-3",
                "Set",
                Force::None,
                state("rc-c3", RecorderState::Set(vec!["⌥".into(), "Space".into()])),
            ));
        self.section("Shortcut recorder", card)
    }

    /// A row with no hint, key or extras, as the board's plain `.row`.
    fn plain_row(&self, id: &'static str, name: &'static str, control: AnyElement) -> SettingsRowProps {
        SettingsRowProps {
            id: id.into(),
            name: name.into(),
            description: None,
            label: div().child(name).into_any_element(),
            tall: false,
            disabled: false,
            changed: false,
            project: None,
            chip: None,
            info: None,
            key_text: None,
            key_name: SharedString::default(),
            copy_label: SharedString::default(),
            on_copy: None,
            dormant: true,
            copied: false,
            hint: None,
            under: None,
            on_reset: None,
            reset_label: SharedString::default(),
            extra: None,
            control,
        }
    }

    pub(super) fn rows(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let none: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)> = Rc::new(|_, _, _| {});
        let keyed = SettingsRowProps {
            key_text: Some("scrollback_lines".into()),
            key_name: "scrollback_lines".into(),
            copy_label: "Copy Key".into(),
            dormant: false,
            ..self.plain_row("kr-key", "Scrollback", self.face("10,000 lines", false, Force::None))
        };
        let changed = SettingsRowProps {
            changed: true,
            key_text: Some("Copied".into()),
            key_name: "minimap".into(),
            copy_label: "Copy Key".into(),
            copied: true,
            dormant: false,
            on_reset: Some(none),
            reset_label: "Reset to Default".into(),
            ..self.plain_row("kr-changed", "Minimap", self.switch("kr-sw", true, false, Force::None))
        };
        let project = SettingsRowProps {
            project: Some("api".into()),
            disabled: true,
            extra: Some(
                div()
                    .text_size(px(12.0))
                    .text_color(self.t("primary"))
                    .child("Edit in project")
                    .into_any_element(),
            ),
            ..self.plain_row(
                "kr-project",
                "Gaps between split terminals",
                self.switch("kr-sw2", true, true, Force::None),
            )
        };
        let menu = self.panel(
            "sheet-menu-key",
            vec![
                self.hot_item("k1", "Copy Key", true),
                self.hot_item("k2", "Copy as JSON", false),
                self.hot_item("k3", "Reset to Default", false),
            ],
        );
        let right_click = SettingsRowProps {
            tall: true,
            description: Some("The same three, without hovering".into()),
            hint: Some("The same three, without hovering".into()),
            ..self.plain_row("kr-menu", "Right-click a row", menu)
        };
        let card = self
            .card()
            .child(keyed.render(&look))
            .child(changed.render(&look))
            .child(project.render(&look))
            .child(div().h(px(112.0)).child(right_click.render(&look)));
        self.section(
            "Key, changed, reset, set by a project",
            div().flex().flex_col().gap(px(8.0)).child(card).child(self.caption(
                "A row under the pointer shows its key after the label; a click copies it. Hold ⌥ to see every \
                 key on the page. A dot marks a changed value, and the arrow puts the default back. A project’s \
                 value shows its name and can’t be changed here.",
            )),
        )
    }

    pub(super) fn tile_rows(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        struct Sample {
            id: &'static str,
            name: &'static str,
            hint: &'static str,
            chip: Option<(&'static str, bool, bool)>,
            note: Option<&'static str>,
            tile: u32,
            icon: IconName,
        }
        // The tiles carry the agents' own brand colours, as the board draws them.
        let samples = [
            Sample {
                id: "tr-1",
                name: "Claude Code",
                hint: "Exact status through hooks",
                chip: None,
                note: None,
                tile: 0xd97757,
                icon: IconName::Sparkles,
            },
            Sample {
                id: "tr-2",
                name: "Codex CLI",
                hint: "Status read from its output",
                chip: None,
                note: None,
                tile: 0x111111,
                icon: IconName::Terminal,
            },
            Sample {
                id: "tr-3",
                name: "Gemini CLI",
                hint: "Some hook events are missing",
                chip: Some(("Needs repair", false, true)),
                note: None,
                tile: 0x1a73e8,
                icon: IconName::Sparkles,
            },
            Sample {
                id: "tr-4",
                name: "Port watcher",
                hint: "ports · version 1.1.0 · by acme",
                chip: Some(("Changed", false, true)),
                note: Some("Its files changed since you approved it."),
                tile: 0x3b4252,
                icon: IconName::Folder,
            },
            Sample {
                id: "tr-5",
                name: "api",
                hint: "~/Projects/api",
                chip: Some(("Trusted", true, false)),
                note: None,
                tile: 0x2c5d8a,
                icon: IconName::Folder,
            },
            Sample {
                id: "tr-6",
                name: "Claude Code",
                hint: "Skill installed · MCP added",
                chip: None,
                note: None,
                tile: 0xd97757,
                icon: IconName::Sparkles,
            },
        ];
        let mut card = self.card();
        for (i, s) in samples.into_iter().enumerate() {
            let controls = match i {
                0 => MenuFaceProps {
                    value: "Hooks on".into(),
                    lead: Some(div().size(px(7.0)).rounded_full().bg(rgb(0x30d158)).into_any_element()),
                    open: false,
                    disabled: false,
                }
                .render(&look)
                .into_any_element(),
                1 => self.button(s.id, "Add Hooks", true, false, false, Force::None),
                2 => self.button(s.id, "Repair Hooks", true, false, false, Force::None),
                3 => self.button(s.id, "Approve Again…", false, false, false, Force::None),
                4 => self.button(s.id, "Revoke", false, false, false, Force::None),
                _ => div()
                    .flex()
                    .gap(px(8.0))
                    .child(self.button("tr-6a", "Uninstall Skill", false, false, false, Force::None))
                    .child(self.button("tr-6b", "Copy MCP Command", false, false, false, Force::None))
                    .into_any_element(),
            };
            card = card.child(
                TileRowProps {
                    id: s.id.into(),
                    name: s.name.into(),
                    spoken: None,
                    name_color: self.t("text.primary"),
                    tile_fill: rgb(s.tile).into(),
                    brand: None,
                    icon: Some(s.icon.into()),
                    info: None,
                    chip: s.chip.map(|c| c.0.into()),
                    good: s.chip.is_some_and(|c| c.1),
                    warn: s.chip.is_some_and(|c| c.2),
                    danger: false,
                    primary: false,
                    hint: Some(s.hint.into()),
                    note: s.note.map(Into::into),
                    note_color: self.t("status.warning.text"),
                    controls,
                }
                .render(&look),
            );
        }
        self.section("List rows with a tile: one height, controls on one edge", card)
    }

    pub(super) fn hints(&self, cx: &App) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let ui = crate::settings::ui(cx);
        let hinted = SettingsRowProps {
            description: Some("Encrypted, on this Mac only. Off erases what’s saved".into()),
            hint: Some("Encrypted, on this Mac only. Off erases what’s saved".into()),
            ..self.plain_row(
                "hn-1",
                "Restore text after restart",
                self.switch("hn-1c", true, false, Force::None),
            )
        };
        let readback = div()
            .px(px(10.0))
            .py(px(7.0))
            .rounded(px(6.0))
            .bg(self.t("button.background"))
            .text_size(px(11.0))
            .line_height(px(15.0))
            .text_color(self.t("text.muted"))
            .child("DECRQCRA: lets a program read the screen cell by cell, over ssh too.")
            .into_any_element();
        let info = SettingsRowProps {
            info: Some(
                crate::style::setting_info("hn-2".into(), ui, "DECRQCRA".into(), Vec::new())
                    .into_any_element(),
            ),
            under: Some(readback),
            ..self.plain_row("hn-2", "Screen read-back", self.switch("hn-2c", false, false, Force::None))
        };
        let blur = crate::style::notice(
            ui,
            crate::style::NoticeKind::Warning,
            "Without blur the desktop shows sharp behind the text: below 95 % opacity it may not read.",
        )
        .into_any_element();
        let warned = SettingsRowProps {
            under: Some(blur),
            ..self.plain_row(
                "hn-3",
                "Blur behind the window",
                self.switch("hn-3c", false, false, Force::None),
            )
        };
        let card =
            self.card().child(hinted.render(&look)).child(info.render(&look)).child(warned.render(&look));
        self.section("Hint, (i) and a warning under a row", card)
    }

    /// An item of a menu with no check column's worth of state: only `active` (the one under the pointer).
    fn hot_item(&self, id: &'static str, name: &'static str, active: bool) -> AnyElement {
        MenuItemProps {
            id: id.into(),
            name: name.into(),
            hint: None,
            lead: None,
            chosen: false,
            active,
            muted: false,
            destructive: false,
            on_click: None,
        }
        .render(&(self.look)(Force::None))
        .into_any_element()
    }
}
