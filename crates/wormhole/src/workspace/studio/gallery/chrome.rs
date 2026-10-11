//! The chrome's icons: a bare icon, the icon button in its sizes and states, the × that
//! closes a row, and a hit area grown round a small drawing.

use super::*;
use gpui_kit::component::Selectable as _;

pub(super) fn render(_studio: &StudioWindow, env: &Env, _cx: &mut Context<StudioWindow>) -> AnyElement {
    let ui = env.ui;
    let note =
        |text: &'static str| div().flex_1().min_w_0().pl_2().text_caption().text_color(ui.muted).child(text);
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            // A chrome icon on its own, in the text color it stands in.
            div()
                .flex()
                .items_center()
                .gap_3()
                .text_color(ui.muted)
                .child(style::icon(IconName::Folder))
                .child(style::icon(IconName::GitCompare))
                .child(style::icon(IconName::Terminal))
                .child(div().text_caption().child("← icons, 14 pt")),
        )
        .child(
            // Icon buttons, 28 square: hover one for `band.hover` and its tooltip.
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(env.mark(
                    "icon.button",
                    style::icon_button("g-ibtn-refresh", ui, IconName::RefreshCw, "Refresh"),
                ))
                .child(style::icon_button("g-ibtn-plus", ui, IconName::Plus, "Add"))
                .child(
                    style::icon_button("g-ibtn-collapse", ui, IconName::PanelLeft, "Hide sidebar")
                        .in_title_row(),
                )
                .child(
                    style::icon_button("g-ibtn-settings", ui, IconName::Settings, "Settings (⌘,)")
                        .in_title_row(),
                )
                .child(style::icon_button("g-ibtn-band", ui, IconName::Plus, "New terminal here").in_band())
                .child(note("← 14 pt, 16 pt in a title row, a band's + (hover for the fill)")),
        )
        .child(
            // Pressed toggles and an active filter take the primary's tint.
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    style::icon_button("g-ibtn-wrap", ui, IconName::TextWrap, "Wrap long lines")
                        .selected(true),
                )
                .child(
                    style::icon_button("g-ibtn-filter", ui, IconName::Funnel, "Group and filter terminals")
                        .in_title_row()
                        .selected(true)
                        .marked(true),
                )
                .child(note("← pressed, and an active filter with its dot")),
        )
        .child(
            // The title row's Open in… group: an app's own icon in the button's face (here a
            // monogram, where the icon can't be read), its menu's narrow chevron, and the splits,
            // flush with no gaps.
            div()
                .flex()
                .items_center()
                .child(
                    style::icon_button("g-ibtn-open-in", ui, IconName::ExternalLink, "Open folder in Zed")
                        .with_drawing(style::agent_logo(None, "Z", style::TITLE_GLYPH, ui.text))
                        .in_title_row(),
                )
                .child(
                    style::icon_button("g-ibtn-open-menu", ui, IconName::ChevronDown, "Open in another app")
                        .narrow(16.0),
                )
                .child(
                    style::icon_button("g-ibtn-split-right", ui, IconName::Columns2, "Split right")
                        .in_title_row(),
                )
                .child(
                    style::icon_button("g-ibtn-split-down", ui, IconName::Rows2, "Split down").in_title_row(),
                )
                .child(note("← an app's icon, its chevron and the splits, flush")),
        )
        .child(
            // The title row's extras: the branch's status (a dot while the tree has
            // uncommitted changes, commits ahead and behind), the agent's model and context in
            // both meter colors, a port, a pull request and the "+n" behind the ports.
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(env.mark("title.extras", style::branch_status(ui, true, 2, 1)))
                .child(style::agent_gauge(
                    ui,
                    Some("Opus 4.5".into()),
                    Some(style::ContextUse { share: 0.41, warn: false, percent: "41%".into() }),
                ))
                .child(style::agent_gauge(
                    ui,
                    None,
                    Some(style::ContextUse { share: 0.86, warn: true, percent: "86%".into() }),
                ))
                .child(style::context_meter(ui, style::METER_W, 0.6, false))
                .child(style::extra_chip(ui, true, Some((IconName::Globe, ui.text)), "localhost:3000"))
                .child(style::pull_request_chip(
                    ui,
                    style::PrLook {
                        icon: IconName::GitPullRequest,
                        color: ui.success_text,
                        label: "#142".into(),
                        checks: Some((IconName::CircleCheck, ui.success_text)),
                    },
                ))
                .child(style::pull_request_mark(
                    ui,
                    style::PrLook {
                        icon: IconName::GitPullRequestDraft,
                        color: ui.muted,
                        label: "#87".into(),
                        checks: Some((IconName::CircleX, ui.danger_text)),
                    },
                ))
                .child(style::extra_chip(ui, false, None, "+1"))
                .child(note("← the title row's extras: branch status, context, a port, a pull request and its mark on a band")),
        )
        .child(
            // The × that removes or closes what it sits on; shown here without the hover reveal.
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(style::close_x("g-close-plain", ui, style::SelectedFill::Plain).text_color(ui.muted))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px_2()
                        .rounded(ui.radius)
                        .bg(ui.selected)
                        .text_color(ui.selected_text)
                        .child(style::close_x("g-close-selected", ui, style::SelectedFill::Tint)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px_2()
                        .rounded(ui.radius_chip)
                        .bg(ui.selected_chip)
                        .text_color(ui.on_selected)
                        .child(style::close_x("g-close-solid", ui, style::SelectedFill::Solid)),
                )
                .child(note("← × on a row, on a selected tint, and on the active tab")),
        )
        .child(
            // A small drawing keeps its size; the outline shows the target round it.
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(style::hit_area(
                    "g-hit-close",
                    style::CONTROL_H_SM,
                    style::close_x("g-hit-x", ui, style::SelectedFill::Plain),
                    |hit| hit.outlined(ui, ui.border),
                ))
                .child(note("← a 22 pt × in its 28 pt hit area")),
        )
        .child(
            // Buttons at a row's ends pull out by the room round their glyph, so the glyphs line
            // up with the row's 16 pt inset like the text in the rows below.
            div()
                .w_full()
                .max_w(px(320.0))
                .flex()
                .flex_col()
                .bg(ui.panel)
                .outlined(ui, ui.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px_4()
                        .h(px(style::TITLE_ROW_H))
                        .child(
                            style::icon_button("g-flush-left", ui, IconName::PanelLeft, "Hide sidebar")
                                .in_title_row()
                                .flush_left(),
                        )
                        .child(div().flex_1())
                        .child(
                            style::icon_button("g-flush-right", ui, IconName::Settings, "Settings (⌘,)")
                                .in_title_row()
                                .flush_right(),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .px_4()
                        .pb_2()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("x 16")
                        .child("width − 16"),
                ),
        )
        .into_any_element()
}
