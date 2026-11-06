//! The setting row in each of its states, and the controls a row holds, each working on sample state.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::*;

use super::{SIZE_DEFAULT, Sheet, category, parts, specimen};
use crate::select::{Choice, SearchSelect, SelectTrigger};
use crate::style::{self, ResetArrow, RowKeys, SettingParts, SettingState};

pub(super) fn draw(sheet: &Sheet, cx: &App) -> AnyElement {
    let ui = sheet.ui;
    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(category(ui, "Setting rows", "style/setting.rs", states(sheet, cx))) // text: sample (a category's head)
        .child(category(ui, "Row controls", "what a row holds, working on sample state", controls(sheet, cx))) // text: sample (a category's head)
        .into_any_element()
}

/// Rows on one card with a divider between each two, as a page's group draws them.
fn group(sheet: &Sheet, rows: impl IntoIterator<Item = AnyElement>) -> Div {
    style::setting_group(sheet.ui, rows)
}

fn row(sheet: &Sheet, id: &'static str, parts: SettingParts, cx: &App) -> AnyElement {
    style::setting(id, sheet.ui, parts, cx).into_any_element()
}

/// The stepper of the sample size: a value that counts, in a row that shows it differs.
fn size_stepper(sheet: &Sheet) -> AnyElement {
    let size = sheet.now.size;
    let (down, up) = (sheet.state.clone(), sheet.state.clone());
    style::stepper(
        sheet.ui,
        "components-size",
        format!("{size} pt"), // text: sample (a stepper's value with its unit)
        move |_, _, cx| {
            down.update(cx, |s, cx| {
                s.now.size = s.now.size.saturating_sub(1).max(8);
                cx.notify();
            })
        },
        move |_, _, cx| {
            up.update(cx, |s, cx| {
                s.now.size = (s.now.size + 1).min(32);
                cx.notify();
            })
        },
    )
    .into_any_element()
}

fn switch(sheet: &Sheet, id: &'static str, on: bool, flip: fn(&mut super::Now, bool)) -> AnyElement {
    let state = sheet.state.clone();
    style::toggle(id, on, move |on, _, cx| {
        state.update(cx, |s, cx| {
            flip(&mut s.now, on);
            cx.notify();
        })
    })
    .into_any_element()
}

/// The setting row in every state it has.
fn states(sheet: &Sheet, cx: &App) -> Vec<AnyElement> {
    let ui = sheet.ui;
    let now = &sheet.now;
    let sizes_differ = now.size != SIZE_DEFAULT;
    let reset_size: style::Reset = {
        let state = sheet.state.clone();
        Rc::new(move |_, cx| {
            state.update(cx, |s, cx| {
                s.now.size = SIZE_DEFAULT;
                cx.notify();
            })
        })
    };
    let size_state = if sizes_differ { SettingState::Changed } else { SettingState::Default };
    let default = || {
        parts(
            "Blur what's behind the window",    // text: sample (a row's label)
            "Only visible below 100 % opacity", // text: sample (a row's description, behind the (i))
            SettingState::Default,
            switch(sheet, "components-blur", now.switch_on, |now, on| now.switch_on = on),
        )
    };
    let changed = || SettingParts {
        on_reset: Some(reset_size.clone()),
        ..parts(
            "Text size", // text: sample (a row's label)
            "Click − or + : the row marks that it differs, and its arrow puts it back", // text: sample (a row's description)
            size_state.clone(),
            size_stepper(sheet),
        )
    };
    let keys = || SettingParts {
        keys: RowKeys { keys: &["blur", "opacity"], json: |_, _| String::new() },
        ..parts(
            "Row with config keys", // text: sample (a row's label)
            "The key shows under the pointer when Settings → Advanced → Show config keys is on", // text: sample (a row's description)
            SettingState::Default,
            switch(sheet, "components-keys", now.switch_off, |now, on| now.switch_off = on),
        )
    };
    let always = || SettingParts {
        reset_arrow: ResetArrow::Always,
        on_reset: Some(Rc::new(|_, _| {})),
        ..parts(
            "Tab width",                                               // text: sample (a row's label)
            "The arrow sits at the right end, as on the Changed page", // text: sample (a row's description)
            SettingState::Changed,
            style::stepper(ui, "components-tab", "8".into(), |_, _, _| {}, |_, _, _| {}).into_any_element(),
        )
    };
    let tall = || SettingParts {
        tall: true,
        ..parts(
            "Row with several lines",               // text: sample (a row's label)
            "The label sits beside the first line", // text: sample (a row's description)
            SettingState::Default,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::text_field(ui, &now.hint).w(style::FIELD_W))
                .child(style::text_field(ui, &now.filled).w(style::FIELD_W))
                .into_any_element(),
        )
    };
    vec![
        specimen(
            ui,
            "setting_group_head, setting_group_blurb, setting_group · three rows, a divider between each", // text: sample (a specimen's name)
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::setting_group_head(ui, "Window", Some("A note at the right of the head"))) // text: sample (a group's head)
                .child(style::setting_group_blurb(ui, "A paragraph under the head, in muted caption text.")) // text: sample (a group's blurb)
                .child(group(
                    sheet,
                    [
                        row(sheet, "components-group-a", default(), cx),
                        row(sheet, "components-group-b", changed(), cx),
                        row(sheet, "components-group-c", keys(), cx),
                    ],
                )),
        )
        .into_any_element(),
        specimen(
            ui,
            "setting · Default, with the (i)", // text: sample (a developer label)
            group(sheet, [row(sheet, "components-default", default(), cx)]),
        ) // text: sample (a specimen's name)
        .into_any_element(),
        specimen(
            ui,
            "setting · Changed, the arrow on hover", // text: sample (a developer label)
            group(sheet, [row(sheet, "components-changed", changed(), cx)]),
        ) // text: sample (a specimen's name)
        .into_any_element(),
        specimen(
            ui,
            "setting · a chip after the label", // text: sample (a specimen's name)
            group(
                sheet,
                [row(
                    sheet,
                    "components-chip",
                    SettingParts {
                        chip: Some("Beta".into()), // text: sample (a chip after a label)
                        ..parts(
                            "Liquid Glass",                              // text: sample (a row's label)
                            "A chip of the moment sits after the label", // text: sample (a row's description)
                            SettingState::Default,
                            switch(sheet, "components-chip-switch", now.switch_off, |now, on| {
                                now.switch_off = on
                            }),
                        )
                    },
                    cx,
                )],
            ),
        )
        .into_any_element(),
        specimen(
            ui,
            "setting · Project, with an extra link", // text: sample (a specimen's name)
            group(
                sheet,
                [row(
                    sheet,
                    "components-project",
                    SettingParts {
                        extra: Some(
                            style::link_button("components-edit", ui, "Edit in project", false) // text: sample (a developer label)
                                .into_any_element(),
                        ), // text: sample (a link)
                        ..parts(
                            "Theme",                                                                // text: sample (a row's label)
                            "Set by api-server/.wormhole/theme.json while this terminal is active", // text: sample (a row's description)
                            SettingState::Project("api-server".into()), // text: sample (a project's name)
                            style::toggle("components-project-switch", true, |_, _, _| {}).into_any_element(),
                        )
                    },
                    cx,
                )],
            ),
        )
        .into_any_element(),
        specimen(
            ui,
            "setting · Disabled, with a visible hint", // text: sample (a specimen's name)
            group(
                sheet,
                [row(
                    sheet,
                    "components-disabled",
                    SettingParts {
                        visible_hint: true,
                        ..parts(
                            "Terminal layout",                // text: sample (a row's label)
                            "No layout.json in this project", // text: sample (a row's description)
                            SettingState::Disabled,
                            style::toggle("components-disabled-switch", false, |_, _, _| {})
                                .disabled(true)
                                .into_any_element(),
                        )
                    },
                    cx,
                )],
            ),
        )
        .into_any_element(),
        specimen(
            ui,
            "setting · a search hit in the label", // text: sample (a specimen's name)
            group(
                sheet,
                [row(
                    sheet,
                    "components-found",
                    SettingParts {
                        found: Some(0..4),
                        ..parts(
                            "Wrap long lines",                     // text: sample (a row's label)
                            "Otherwise the diff scrolls sideways", // text: sample (a row's description)
                            SettingState::Default,
                            style::toggle("components-found-switch", false, |_, _, _| {}).into_any_element(),
                        )
                    },
                    cx,
                )],
            ),
        )
        .into_any_element(),
        specimen(
            ui,
            "setting · RowKeys, the config key's line", // text: sample (a developer label)
            group(sheet, [row(sheet, "components-keys-row", keys(), cx)]),
        ) // text: sample (a specimen's name)
        .into_any_element(),
        specimen(
            ui,
            "setting · ResetArrow::Always", // text: sample (a developer label)
            group(sheet, [row(sheet, "components-always", always(), cx)]),
        ) // text: sample (a specimen's name)
        .into_any_element(),
        specimen(
            ui,
            "setting · tall, a control of several lines", // text: sample (a developer label)
            group(sheet, [row(sheet, "components-tall", tall(), cx)]),
        ) // text: sample (a specimen's name)
        .into_any_element(),
    ]
}

/// The controls a row holds, each in a row of its own and working.
fn controls(sheet: &Sheet, cx: &App) -> Vec<AnyElement> {
    let ui = sheet.ui;
    let now = &sheet.now;
    let plain = |id: &'static str, label: &'static str, control: AnyElement| {
        row(sheet, id, parts(label, "A sample row", SettingState::Default, control), cx) // text: sample (a row's description)
    };
    let segments = |id: &'static str,
                    options: &[(Option<IconName>, &'static str)],
                    current: usize,
                    pick: fn(&mut super::Now, usize)| {
        let mut control = style::segmented_form(ui);
        for (ix, (icon, label)) in options.iter().enumerate() {
            let state = sheet.state.clone();
            control = control.child(
                style::form_segment((id, ix), ui, *icon, *label, ix == current).on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        pick(&mut s.now, ix);
                        cx.notify();
                    })
                }),
            );
        }
        control.into_any_element()
    };
    let checks = (0..3).fold(div().flex().flex_col().gap_2(), |column, ix| {
        let state = sheet.state.clone();
        column.child(
            style::checkbox(("components-check", ix), ui, now.checks[ix])
                .in_form(["Show on every page", "Only on this page", "Never"][ix]) // text: sample (a checkbox's label)
                .on_change(move |on, _, cx| {
                    state.update(cx, |s, cx| {
                        s.now.checks[ix] = on;
                        cx.notify();
                    })
                }),
        )
    });
    let panel = (0..3).fold(style::segmented(ui), |control, ix| {
        let state = sheet.state.clone();
        control.child(
            style::segment(
                ("components-panel", ix),
                ui,
                ["Split", "Unified", "Both"][ix], // text: sample (a developer label)
                ix == now.panel_segment,
            ) // text: sample (a segment)
            .on_click(move |_, _, cx| {
                state.update(cx, |s, cx| {
                    s.now.panel_segment = ix;
                    cx.notify();
                })
            }),
        )
    });
    let slider = style::slider_control(
        ui,
        style::slider(ui, &now.slider),
        format!("{} %", now.slider_value), // text: sample (a slider's value)
        &["30", "65", "100"],
        false,
    );
    let compact = style::slider_control(ui, style::slider(ui, &now.slider), "80 %", &["30", "100"], true); // text: sample (a slider's value)
    let select = {
        let (open, pick) = (sheet.state.clone(), sheet.state.clone());
        let width = style::SELECT_W;
        SearchSelect::new(
            "components-select",
            ui,
            SelectTrigger::new("components-select-trigger", ui, now.select_value.clone()).width(px(width)),
            &now.menu_search,
            cx,
        )
        .options(
            ["Tokyo Night", "Nord", "Dracula", "Solarized"] // text: sample (a select's choices)
                .map(|name| Choice::new(name.to_string(), name))
                .to_vec(),
        )
        .current(now.select_value.clone())
        .open(now.select_open)
        .without_search()
        .anchor(Anchor::TopRight)
        .menu_width(px(width))
        .on_open_change(move |is_open, _, cx| {
            open.update(cx, |s, cx| {
                s.now.select_open = is_open;
                cx.notify();
            })
        })
        .on_pick(move |value, _, cx| {
            let value = value.clone();
            pick.update(cx, |s, cx| {
                s.now.select_value = value;
                s.now.select_open = false;
                cx.notify();
            })
        })
        .into_popover()
        .into_any_element()
    };
    let link = style::link_button("components-link", ui, "Edit in Fonts", true).into_any_element(); // text: sample (a link)
    vec![
        specimen(
            ui,
            "toggle · on, off (a switch)", // text: sample (a specimen's name)
            group(
                sheet,
                [
                    plain(
                        "components-on",
                        "On", // text: sample (a developer label)
                        switch(sheet, "components-toggle-on", now.switch_on, |now, on| now.switch_on = on),
                    ), // text: sample (a row's label)
                    plain(
                        "components-off",
                        "Off", // text: sample (a developer label)
                        switch(sheet, "components-toggle-off", now.switch_off, |now, on| now.switch_off = on),
                    ), // text: sample (a row's label)
                ],
            ),
        )
        .into_any_element(),
        specimen(ui, "checkbox · in_form", checks).into_any_element(), // text: sample (a specimen's name)
        specimen(
            ui,
            "segmented_form · text, with icons, many options", // text: sample (a specimen's name)
            group(
                sheet,
                [
                    plain(
                        "components-seg-text", // text: not shown (a row's id)
                        "Diff layout",         // text: sample (a row's label)
                        segments(
                            "components-seg-a",
                            &[(None, "Split"), (None, "Unified")], // text: sample (a developer label)
                            now.segment,
                            |now, ix| now.segment = ix,
                        ), // text: sample (segments)
                    ),
                    plain(
                        "components-seg-icons", // text: not shown (a row's id)
                        "Appearance",           // text: sample (a row's label)
                        segments(
                            "components-seg-b", // text: not shown (a row's id)
                            &[
                                (Some(IconName::Moon), "Dark"), // text: sample (a developer label)
                                (Some(IconName::Sun), "Light"), // text: sample (a developer label)
                                (Some(IconName::Monitor), "Match system"), // text: sample (a developer label)
                            ], // text: sample (segments)
                            now.segment_icons,
                            |now, ix| now.segment_icons = ix,
                        ),
                    ),
                    plain(
                        "components-seg-many", // text: not shown (a row's id)
                        "Scrollback",          // text: sample (a row's label)
                        segments(
                            "components-seg-c", // text: not shown (a row's id)
                            &[(None, "1k"), (None, "10k"), (None, "100k"), (None, "1M"), (None, "Unlimited")], // text: sample (segments)
                            now.segment_many,
                            |now, ix| now.segment_many = ix,
                        ),
                    ),
                ],
            ),
        )
        .into_any_element(),
        specimen(ui, "segmented · segment (a panel's)", panel).into_any_element(), // text: sample (a specimen's name)
        specimen(
            ui,
            "stepper · with a unit", // text: sample (a developer label)
            group(sheet, [plain("components-stepper", "Font size", size_stepper(sheet))]), // text: sample (a developer label)
        ) // text: sample (a specimen's name)
        .into_any_element(),
        specimen(
            ui,
            "slider_control · with ticks and a value, compact", // text: sample (a specimen's name)
            group(
                sheet,
                [
                    plain("components-slider", "Background opacity", slider.into_any_element()), // text: sample (a developer label)
                    plain("components-slider-compact", "Compact", compact.into_any_element()), // text: sample (a developer label)
                ],
            ), // text: sample (a row's label)
        )
        .into_any_element(),
        specimen(
            ui,
            "SearchSelect · closed, open on click", // text: sample (a specimen's name)
            group(sheet, [plain("components-select-row", "Theme", select)]), // text: sample (a row's label)
        )
        .into_any_element(),
        specimen(
            ui,
            "link_button · with an arrow", // text: sample (a developer label)
            group(sheet, [plain("components-link-row", "Font", link)]), // text: sample (a developer label)
        ) // text: sample (a specimen's name)
        .into_any_element(),
    ]
}
