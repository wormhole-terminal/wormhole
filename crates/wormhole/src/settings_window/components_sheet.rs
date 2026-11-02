//! The component sheet (devctl `settings-components`): every compiled component in its sample cases, laid
//! out as the design's Components board is (three columns, 40 apart), with the states a pointer would make
//! (hover, pressed, focus) forced through [`Force`], so a screenshot of the window can be held against the
//! board without a pointer. Not shipped: the module is built with devctl only.

use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::settings::SettingsStore;
use crate::theme_bridge::token;
use crate::ui::{
    CheckboxControlProps, Force, FormButtonProps, Look, MenuFaceProps, MenuItemProps, MenuPanelProps,
    MoreRowProps, NavEntry, NoticeBarProps, PageTitleProps, SettingsNavProps, SettingsRowProps,
    SidebarProfileProps, SolidChipProps, StepperProps, SwitchControlProps, TileRowProps,
};

/// Where the board puts each section (x, y, width), so each is compared on its own whatever the others take.
const SPOTS: &[(&str, f32, f32, f32)] = &[
    ("Sidebar top, page title, headings", 40.0, 40.0, 413.3),
    ("Switch", 40.0, 349.0, 413.3),
    ("Menu: closed, open, with a destructive item", 40.0, 520.0, 413.3),
    ("Menus: icons, a separator, an action, a project’s own value", 40.0, 843.0, 413.3),
    ("Checkbox", 40.0, 1037.0, 413.3),
    ("Buttons", 40.0, 1246.0, 413.3),
    ("Row that opens more, text field, value", 40.0, 1383.0, 413.3),
    ("Hint, (i) and a warning under a row", 40.0, 1728.0, 413.3),
    ("Key, changed, reset, set by a project", 493.3, 40.0, 413.3),
    ("Status chips: solid, white text", 493.3, 387.0, 413.3),
    ("List rows with a tile: one height, controls on one edge", 493.3, 510.0, 413.3),
    ("Notices: solid", 946.7, 40.0, 413.3),
    ("Hover and keyboard focus", 946.7, 1716.0, 413.3),
    ("Sidebar rows", 946.7, 1888.0, 413.3),
];

pub(super) fn render(from: f32, cx: &App) -> AnyElement {
    let store = cx.global::<SettingsStore>();
    let tokens = &store.tokens;
    let t = |name: &str| token(tokens, name);
    let look = |force| Look {
        tokens,
        code_font: crate::fonts::code_font(cx).0,
        borders: crate::settings::ui(cx).borders,
        force,
    };
    let cards = Cards { t: &t, look: &look, from };
    div()
        .size_full()
        .overflow_hidden()
        .bg(t("surface.window"))
        .text_color(t("text.primary"))
        .relative()
        .text_size(px(13.0))
        .children(
            [
                cards.sidebar_top(),
                cards.switches(),
                cards.menus(),
                cards.checkboxes(),
                cards.buttons(),
                cards.more_rows(),
                cards.hints(cx),
                cards.rows(),
                cards.chips(),
                cards.tile_rows(),
                cards.notices(cx),
                cards.forced(),
                cards.nav(),
            ]
            .into_iter()
            .flatten(),
        )
        .into_any_element()
}

struct Cards<'a> {
    t: &'a dyn Fn(&str) -> Hsla,
    look: &'a dyn Fn(Force) -> Look<'a>,
    /// The board's y that is at the window's top.
    from: f32,
}

impl Cards<'_> {
    fn t(&self, name: &str) -> Hsla {
        (self.t)(name)
    }

    /// A titled section, as the board's `.sec`.
    fn section(&self, title: &str, body: impl IntoElement) -> Vec<AnyElement> {
        let (x, y, w) = SPOTS.iter().find(|s| s.0 == title).map_or((0.0, 0.0, 400.0), |s| (s.1, s.2, s.3));
        vec![
            div()
                .absolute()
                .left(px(x))
                .top(px(y - self.from))
                .w(px(w))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .ml(px(4.0))
                        .text_size(px(13.0))
                        .line_height(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(self.t("text.muted"))
                        .child(SharedString::from(title.to_string())),
                )
                .child(body)
                .into_any_element(),
        ]
    }

    /// The board's `.card`: the band's wash, 10 round.
    fn card(&self) -> Div {
        div().bg(self.t("band.hover")).rounded(px(10.0)).overflow_hidden()
    }

    fn row(&self, id: &'static str, name: &'static str, force: Force, control: AnyElement) -> AnyElement {
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
        .render(&(self.look)(force))
        .into_any_element()
    }

    fn switch(&self, id: &'static str, on: bool, disabled: bool, force: Force) -> AnyElement {
        SwitchControlProps { id: id.into(), label: id.into(), on, disabled, on_click: None }
            .render(&(self.look)(force))
            .into_any_element()
    }

    fn face(&self, value: &'static str, open: bool, force: Force) -> AnyElement {
        MenuFaceProps { value: value.into(), lead: None, open, disabled: false }
            .render(&(self.look)(force))
            .into_any_element()
    }

    /// A menu item with a 14 square in front of its name (an app's icon).
    fn item_lead(&self, id: &'static str, name: &'static str, chosen: bool, fill: &str) -> AnyElement {
        MenuItemProps {
            id: id.into(),
            name: name.into(),
            hint: None,
            lead: Some(div().size(px(14.0)).rounded(px(3.0)).bg(self.t(fill)).into_any_element()),
            chosen,
            active: chosen,
            muted: false,
            destructive: false,
            on_click: None,
        }
        .render(&(self.look)(Force::None))
        .into_any_element()
    }

    fn item(
        &self,
        id: &'static str,
        name: &'static str,
        chosen: bool,
        destructive: bool,
        force: Force,
    ) -> AnyElement {
        MenuItemProps {
            id: id.into(),
            name: name.into(),
            hint: None,
            lead: None,
            chosen,
            active: chosen,
            muted: false,
            destructive,
            on_click: None,
        }
        .render(&(self.look)(force))
        .into_any_element()
    }

    fn panel(&self, id: &'static str, items: Vec<AnyElement>) -> AnyElement {
        MenuPanelProps {
            id: id.into(),
            label: SharedString::default(),
            rows: div().flex().flex_col().children(items).into_any_element(),
        }
        .render(&(self.look)(Force::None))
        .into_any_element()
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        primary: bool,
        destructive: bool,
        disabled: bool,
        force: Force,
    ) -> AnyElement {
        FormButtonProps { id: id.into(), label: label.into(), primary, destructive, disabled, on_click: None }
            .render(&(self.look)(force))
            .into_any_element()
    }

    fn sidebar_top(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let profile = SidebarProfileProps {
            id: "sheet-profile".into(),
            label: "Default".into(),
            letter: "D".into(),
            name: "Default".into(),
            caption: "Profile".into(),
            selected: false,
            hits: None,
            on_click: None,
        }
        .render(&look);
        let panel = div()
            .w(px(240.0))
            .px(px(12.0))
            .py(px(16.0))
            .rounded(px(10.0))
            .bg(self.t("sidebar.background"))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .pl(px(8.0))
                    // The OS's own lights, which the board draws as three fixed colours.
                    .children(
                        [0xff5f57_u32, 0xfebc2e, 0x28c840]
                            .map(|c| div().size(px(12.0)).rounded_full().bg(rgb(c))),
                    ),
            )
            .child(
                div()
                    .h(px(28.0))
                    .px(px(10.0))
                    .rounded(px(7.0))
                    .bg(self.t("input.background"))
                    .text_color(self.t("text.muted"))
                    .flex()
                    .items_center()
                    .child("Search"),
            )
            .child(profile);
        let title =
            self.card().p(px(14.0)).flex().flex_col().gap(px(12.0)).child(
                PageTitleProps { title: "Motion".into(), badge: Some("Preview".into()) }.render(&look),
            );
        self.section(
            "Sidebar top, page title, headings",
            div().flex().flex_col().gap(px(8.0)).child(panel).child(title),
        )
    }

    fn switches(&self) -> Vec<AnyElement> {
        let card = self
            .card()
            .child(self.row("sw-on", "On", Force::None, self.switch("sw-on-c", true, false, Force::None)))
            .child(self.row("sw-off", "Off", Force::None, self.switch("sw-off-c", false, false, Force::None)))
            .child(self.row(
                "sw-dis",
                "Disabled",
                Force::None,
                self.switch("sw-dis-c", true, true, Force::None),
            ));
        self.section("Switch", card)
    }

    fn menus(&self) -> Vec<AnyElement> {
        let open = div()
            .relative()
            .child(self.face("Ask", true, Force::None))
            .child(div().absolute().top(px(28.0)).right_0().child(self.panel(
                "sheet-menu-open",
                vec![
                    self.item("m1", "Ask", true, false, Force::None),
                    self.item("m2", "Allow", false, false, Force::None),
                    self.item("m3", "Deny", false, false, Force::None),
                ],
            )))
            .into_any_element();
        let actions = div()
            .relative()
            .child(self.face("Hooks on", false, Force::None))
            .child(div().absolute().top(px(28.0)).right_0().child(self.panel(
                "sheet-menu-actions",
                vec![
                    self.item("a1", "Repair Hooks", false, false, Force::None),
                    self.item("a2", "Remove Hooks", false, true, Force::None),
                ],
            )))
            .into_any_element();
        let card = self
            .card()
            .child(self.row(
                "mn-closed",
                "Closed",
                Force::None,
                self.face("10,000 lines", false, Force::None),
            ))
            .child(div().h(px(128.0)).child(self.row("mn-open", "Open", Force::None, open)))
            .child(div().h(px(104.0)).child(self.row("mn-actions", "Actions", Force::None, actions)));
        let two = self
            .card()
            .p(px(14.0))
            .flex()
            .gap(px(16.0))
            .items_start()
            .child(self.panel(
                "sheet-menu-icons",
                vec![
                    self.item_lead("i1", "Zed", true, "settings.nav.tile.terminal"),
                    self.item_lead("i2", "Visual Studio Code", false, "primary"),
                    self.item_lead("i3", "Finder", false, "text.faint"),
                    div().h(px(1.0)).mx(px(6.0)).my(px(4.0)).bg(self.t("border")).into_any_element(),
                    MenuItemProps {
                        id: "i4".into(),
                        name: "Choose app…".into(),
                        hint: None,
                        lead: None,
                        chosen: false,
                        active: false,
                        muted: true,
                        destructive: false,
                        on_click: None,
                    }
                    .render(&(self.look)(Force::None))
                    .into_any_element(),
                ],
            ))
            .child(self.panel(
                "sheet-menu-own",
                vec![
                    self.item("o1", "Default (on)", true, false, Force::None),
                    self.item("o2", "On", false, false, Force::None),
                    self.item("o3", "Off", false, false, Force::None),
                ],
            ));
        let mut out = self.section("Menu: closed, open, with a destructive item", card);
        out.extend(self.section("Menus: icons, a separator, an action, a project’s own value", two));
        out
    }

    fn checkboxes(&self) -> Vec<AnyElement> {
        let boxed = |id: &'static str, on| {
            CheckboxControlProps { id: id.into(), label: id.into(), on, on_click: None }
                .render(&(self.look)(Force::None))
                .into_any_element()
        };
        let card = self
            .card()
            .child(self.row("cb-on", "Ticked", Force::None, boxed("cb-on-c", true)))
            .child(self.row("cb-off", "Not ticked", Force::None, boxed("cb-off-c", false)));
        self.section("Checkbox", card)
    }

    fn buttons(&self) -> Vec<AnyElement> {
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
            .child(self.button("b4", "Make Default", false, false, true, Force::None));
        self.section("Buttons", card)
    }

    fn more_rows(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let card = self.card().child(
            MoreRowProps {
                id: "sheet-more".into(),
                label: "Approved programs".into(),
                value: Some("None".into()),
                open: false,
                closed: true,
                on_click: None,
            }
            .render(&look),
        );
        let stepper = StepperProps {
            id: "sheet-stepper".into(),
            label: "Font size".into(),
            smaller: "Smaller".into(),
            larger: "Larger".into(),
            value: "14 pt".into(),
            on_smaller: None,
            on_larger: None,
        }
        .render(&look);
        self.section(
            "Row that opens more, text field, value",
            div().flex().flex_col().gap(px(8.0)).child(card).child(stepper),
        )
    }

    fn rows(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let keyed = |id: &'static str, name: &'static str, changed: bool, control: AnyElement| {
            SettingsRowProps {
                id: id.into(),
                name: name.into(),
                description: None,
                label: div().child(name).into_any_element(),
                tall: false,
                disabled: false,
                changed,
                project: None,
                chip: None,
                info: None,
                key_text: Some("scrollback_lines".into()),
                key_name: "scrollback_lines".into(),
                copy_label: "Copy Key".into(),
                on_copy: None,
                dormant: false,
                copied: false,
                hint: None,
                under: None,
                on_reset: None,
                reset_label: "Reset to Default".into(),
                extra: None,
                control,
            }
            .render(&look)
            .into_any_element()
        };
        let card = self
            .card()
            .child(keyed("kr-key", "Scrollback", false, self.face("10,000 lines", false, Force::None)))
            .child(keyed("kr-changed", "Minimap", true, self.switch("kr-sw", true, false, Force::None)));
        self.section("Key, changed, reset, set by a project", card)
    }

    fn chips(&self) -> Vec<AnyElement> {
        let chip = |text: &'static str, good, warn, danger, primary| {
            SolidChipProps { text: text.into(), good, warn, danger, primary }
                .render(&(self.look)(Force::None))
                .into_any_element()
        };
        let card = self
            .card()
            .p(px(14.0))
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .child(chip("On", true, false, false, false))
            .child(chip("Trusted", true, false, false, false))
            .child(chip("Off", false, false, false, false))
            .child(chip("Not trusted", false, false, false, false))
            .child(chip("Needs code", false, false, false, false))
            .child(chip("Changed", false, true, false, false))
            .child(chip("Needs repair", false, true, false, false))
            .child(chip("Can't be enabled", false, false, true, false))
            .child(chip("Preview", false, false, false, true));
        self.section("Status chips: solid, white text", card)
    }

    fn tile_rows(&self) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let row = |id: &'static str,
                   name: &'static str,
                   hint: &'static str,
                   chip: Option<&'static str>,
                   action: &'static str,
                   primary: bool| {
            TileRowProps {
                id: id.into(),
                name: name.into(),
                spoken: None,
                name_color: self.t("text.primary"),
                tile_fill: self.t("settings.nav.tile.agents"),
                brand: None,
                icon: Some(IconName::Sparkles.into()),
                info: None,
                chip: chip.map(Into::into),
                good: false,
                warn: chip.is_some(),
                danger: false,
                primary: false,
                hint: Some(hint.into()),
                note: None,
                note_color: self.t("text.muted"),
                controls: self.button(id, action, primary, false, false, Force::None),
            }
            .render(&look)
            .into_any_element()
        };
        let card = self
            .card()
            .child(row("tr-1", "Codex CLI", "Status read from its output", None, "Add Hooks", true))
            .child(row(
                "tr-2",
                "Gemini CLI",
                "Some hook events are missing",
                Some("Needs repair"),
                "Repair Hooks",
                true,
            ));
        self.section("List rows with a tile: one height, controls on one edge", card)
    }

    fn hints(&self, cx: &App) -> Vec<AnyElement> {
        let look = (self.look)(Force::None);
        let ui = crate::settings::ui(cx);
        let plain = |id: &'static str,
                     name: &'static str,
                     description: Option<&'static str>,
                     info: Option<AnyElement>,
                     under: Option<AnyElement>,
                     on: bool| {
            let info_none = info.is_none();
            SettingsRowProps {
                id: id.into(),
                name: name.into(),
                description: description.map(Into::into),
                label: div().child(name).into_any_element(),
                tall: false,
                disabled: false,
                changed: false,
                project: None,
                chip: None,
                info,
                key_text: None,
                key_name: SharedString::default(),
                copy_label: SharedString::default(),
                on_copy: None,
                dormant: true,
                copied: false,
                hint: description.filter(|_| info_none).map(Into::into),
                under,
                on_reset: None,
                reset_label: SharedString::default(),
                extra: None,
                control: self.switch(id, on, false, Force::None),
            }
            .render(&look)
            .into_any_element()
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
        let blur = crate::style::notice(
            ui,
            crate::style::NoticeKind::Warning,
            "Without blur the desktop shows sharp behind the text: below 95 % opacity it may not read.",
        )
        .into_any_element();
        let card = self
            .card()
            .child(plain(
                "hn-1",
                "Restore text after restart",
                Some("Encrypted, on this Mac only. Off erases what’s saved"),
                None,
                None,
                true,
            ))
            .child(plain(
                "hn-2",
                "Screen read-back",
                Some("DECRQCRA"),
                Some(
                    crate::style::setting_info("hn-2".into(), ui, "DECRQCRA".into(), Vec::new())
                        .into_any_element(),
                ),
                Some(readback),
                false,
            ))
            .child(plain("hn-3", "Blur behind the window", None, None, Some(blur), false));
        self.section("Hint, (i) and a warning under a row", card)
    }

    fn notices(&self, cx: &App) -> Vec<AnyElement> {
        let notice = |id: &'static str, text: &'static str, warning: bool, danger: bool| {
            NoticeBarProps {
                id: id.into(),
                text: text.into(),
                warning,
                danger,
                actions: (id == "n2").then(|| {
                    crate::style::notice_button(
                        "n2-open",
                        crate::settings::ui(cx),
                        crate::style::NoticeKind::Warning,
                        cx,
                    )
                    .label("Open")
                    .into_any_element()
                }),
            }
            .render(&(self.look)(Force::None))
            .into_any_element()
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(notice("n1", "Development build: updates only work in released builds.", false, false))
            .child(notice(
                "n2",
                "Off in System Settings: Wormhole can’t show banners until you allow them.",
                true,
                false,
            ))
            .child(notice(
                "n3",
                "Wormhole couldn't read its key from the Keychain, so extensions stay off.",
                false,
                true,
            ));
        self.section("Notices: solid", body)
    }

    /// The board's "Hover and keyboard focus": the same controls in the states a pointer or the keys make.
    fn forced(&self) -> Vec<AnyElement> {
        let card = self
            .card()
            .child(self.row(
                "fc-hover",
                "Row under the pointer",
                Force::Hover,
                self.switch("fc-1", false, false, Force::None),
            ))
            .child(self.row(
                "fc-sw",
                "Focus on a switch",
                Force::None,
                self.switch("fc-2", true, false, Force::Focus),
            ))
            .child(self.row(
                "fc-btn",
                "Focus on a control",
                Force::None,
                self.button("fc-3", "Show in Finder", false, false, false, Force::Focus),
            ));
        let pressed = self
            .card()
            .child(self.row(
                "fp-btn",
                "Button pressed",
                Force::None,
                self.button("fp-1", "Show in Finder", false, false, false, Force::Pressed),
            ))
            .child(self.row(
                "fh-btn",
                "Button hover",
                Force::None,
                self.button("fp-2", "Show in Finder", false, false, false, Force::Hover),
            ));
        self.section(
            "Hover and keyboard focus",
            div().flex().flex_col().gap(px(8.0)).child(card).child(pressed),
        )
    }

    fn nav(&self) -> Vec<AnyElement> {
        let item = |key: usize, name: &'static str, tile: &str, force: Force, selected: bool| {
            let entry = NavEntry {
                key,
                is_item: true,
                name: name.into(),
                spoken: name.into(),
                icon: Some(IconName::Sparkles.into()),
                tile: self.t(tile),
                selected,
                ..NavEntry::default()
            };
            (entry, force)
        };
        let mut rows = div().flex().flex_col().gap(px(2.0));
        for (entry, force) in [
            item(0, "Changed", "settings.nav.tile.changed", Force::None, false),
            item(1, "Changes & diffs", "settings.nav.tile.git", Force::None, false),
            item(2, "Notifications (hover)", "settings.nav.tile.notifications", Force::Hover, false),
            item(3, "Coding agents", "settings.nav.tile.agents", Force::None, true),
        ] {
            rows = rows.child(SettingsNavProps { rows: vec![entry] }.render(&(self.look)(force)));
        }
        let body = self.card().p(px(8.0)).bg(self.t("sidebar.background")).w(px(240.0)).child(rows);
        self.section("Sidebar rows", body)
    }
}
