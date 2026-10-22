//! The terminal's title row: the breadcrumb of where
//! the selected terminal is and what it does (`breadcrumb`), Open in… and the two splits flush at
//! the right end, the project's tint behind them and the line under the row.
//!
//! It has its own row above the terminal's surface and never overlaps it. The row's height is [`style::TITLE_ROW_H`]; the line, when it is drawn, is the one
//! border width under it. An empty stretch of it is the window's title bar: dragging moves the
//! window and a double-click does what the system setting says (zoom, or minimize).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::platform::open_in::{Installed, Resolved};
use wormhole_core::project::ProjectColor;
use wormhole_core::view::layout::title_row::{PathStep, RowFit};
use wormhole_core::view::panes::Axis;

use crate::menubar::model::KeyEquivalent;
use crate::platform::{self, Anchor, Icon, Item, Menu};
use crate::settings::{UiColors, store_in, ui_in};
use crate::style::{self, Edges};
use crate::terminal_view::Status;
use crate::theme_bridge::token;
use crate::workspace::Workspace;
use crate::workspace::chrome::TitleDrag as _;
use crate::workspace::folder_color;
use crate::workspace::open_in::{self, OpenIn};
use crate::workspace::title_extras::ExtraFacts;

/// What the title row says about the terminal it is for, gathered once per frame.
pub(super) struct Facts {
    pub(super) id: i64,
    /// Where the terminal is, and how that reads (`~/Projects/tube`).
    pub(super) folder: Option<PathBuf>,
    pub(super) shown: Option<String>,
    pub(super) status: Status,
    pub(super) agent: Option<AgentId>,
    /// Something tells the status: an agent, or a program that reports its own.
    pub(super) told: bool,
    /// The terminal's name: its title, else what runs in it.
    pub(super) label: SharedString,
    /// How it is doing after the name ("working · 4m"); nothing while it is idle.
    pub(super) words: Option<String>,
    /// The branch of the folder: `Some(None)` is a detached HEAD.
    pub(super) branch: Option<Option<String>>,
    /// For a terminal in a submodule, whose it is: "in terminal".
    pub(super) parent: Option<String>,
    /// The extras beside it (`title_extras`): its status, the agent's model and context, the ports
    /// and the pull request, each only if its setting is on and there is something to say.
    pub(super) extras: ExtraFacts,
}

/// The project's colour as the row draws it.
pub(super) struct Tint {
    pub(super) color: ProjectColor,
    /// How strong it is at the left edge: 36 %, less where the text would not read.
    pub(super) peak: f32,
    /// The line that takes its place under Increase Contrast.
    pub(super) line: Hsla,
}

/// Everything the title row draws, worked out once per frame: the terminal's facts, what the narrow
/// row gives up (core's `fit_row`), the app a click on Open in… opens, and the project's tint.
pub(super) struct TitlePlan {
    pub(super) facts: Facts,
    pub(super) steps: Vec<PathStep>,
    pub(super) fit: RowFit,
    pub(super) app: Resolved,
    pub(super) tint: Option<Tint>,
}

/// The width of Open in… (the app's icon and its chevron), the two splits, and the one `⋯` they
/// fold into; each with the gap that separates it from the breadcrumb.
pub(super) const ROW_GAP: f32 = 4.0;
/// The title row is a bar this far in from the column's sides and the window's top, centred on the
/// traffic lights, with as much under it before the terminal's surface ([`SURFACE_TOP`]).
pub(super) const BAR_INSET: f32 = 4.0;
/// Where the terminal's surface (or, with terminals as tabs, their bar) starts under the window's top:
/// under the whole title row.
pub(super) const SURFACE_TOP: f32 = style::TITLE_ROW_H;
pub(super) const OPEN_IN_W: f32 = style::HIT_MIN + CHEVRON_W + AFTER_CHEVRON;
const CHEVRON_W: f32 = 16.0;
/// After Open in…'s narrow chevron, so its glyph stands as far from the split's as the row's other
/// glyphs stand from each other (12 pt).
const AFTER_CHEVRON: f32 = 4.0;
pub(super) const SPLITS_W: f32 = 2.0 * style::HIT_MIN;
pub(super) const MORE_W: f32 = style::HIT_MIN;
/// The app's icon in the Open in… button, and in its menu.
const APP_ICON: f32 = style::TITLE_GLYPH;

impl Workspace {
    /// The terminal's title row: the selected terminal's breadcrumb (or the diff's title) and the
    /// buttons at the right end, in its own row above the terminal's surface.
    pub(super) fn render_terminal_title_row(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let (drop_tint, system) = {
            let store = store_in(&self.profile, cx);
            (token(&store.tokens, "state.highlight"), store.system())
        };
        let in_diff = self.diff_view.is_some();
        let plan = (!in_diff).then(|| self.title_plan(window, cx));
        let (left, right) = self.title_row_pads(window);

        // The bar has no fill of its own: its content stands on the window. Under Increase Contrast
        // the group's colour is a line under it.
        let tint = plan.as_ref().and_then(|plan| plan.tint.as_ref());
        let line = tint.filter(|_| system.increase_contrast).map(|tint| tint.line);
        let strip = plan.as_ref().map(|plan| self.render_title_strip(plan, window, cx));
        let extras = plan.as_ref().and_then(|plan| self.render_title_extras(plan, cx));
        let actions = plan.as_ref().map(|plan| self.render_title_actions(plan, cx));
        let bar = div()
            .relative()
            .flex_none()
            .mx(px(BAR_INSET))
            .mt(px(BAR_INSET))
            .mb(px(BAR_INSET))
            .h(px(style::TITLE_ROW_H - 2.0 * BAR_INSET))
            .rounded(ui.radius_lg)
            .overflow_hidden()
            .when_some(line, |d, color| d.rule_bottom_in(ui, color, 2.0))
            .child(
                self.with_title_pads(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .gap_1()
                        .title_drag()
                        // The extras are at this stretch's right end, so no gap of the row's lies between
                        // them and Open in…: the width the row gives way by counts none.
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .children(self.render_diff_title(cx))
                                .children(strip)
                                .when(extras.is_some(), |d| d.child(div().flex_1().min_w_0()))
                                .children(extras),
                        )
                        .children(actions),
                    (left, right),
                ),
            );

        div()
            .id("titlebar")
            // Keep clicks (and the I-beam cursor) from reaching the terminal below;
            // double-click still zooms the window via the drag area.
            .occlude()
            .cursor(CursorStyle::Arrow)
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            // The diff view replaces the terminal, so there is nothing to drop onto then.
            .when(!in_diff, |d| {
                d.drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(drop_tint)).on_drop(cx.listener(
                    |this, files: &ExternalPaths, window, cx| this.drop_files_on_titlebar(files, window, cx),
                ))
            })
            .child(bar)
            .into_any_element()
    }

    /// The title row's padding: `pads` at rest, and following the column's edge while one slides
    /// (`chrome::sliding_title_pads`), so the breadcrumb and the buttons move with it instead of
    /// jumping when it lands.
    fn with_title_pads(&self, row: Div, (left, right): (f32, f32)) -> AnyElement {
        // The pads are measured from the column's edges; the bar is inset from them.
        let inside = |(left, right): (f32, f32)| ((left - BAR_INSET).max(0.0), (right - BAR_INSET).max(0.0));
        let Some(motion) = self.slide_motion() else {
            let (left, right) = inside((left, right));
            return row.pl(px(left)).pr(px(right)).into_any_element();
        };
        row.with_animation(motion.id("slide-title"), motion.animation(), move |d, t| {
            let (left, right) = inside(crate::workspace::chrome::sliding_title_pads(
                motion.edge,
                (left, right),
                motion.offset(t),
            ));
            d.pl(px(left)).pr(px(right))
        })
        .into_any_element()
    }

    /// The tint behind the active terminal's title row: its group's colour, as the sidebar's card
    /// wears it. A terminal in no group (or in a group without a colour) has none. It only
    /// paints, so it needs no trust.
    fn title_tint(&self, cx: &App) -> Option<Tint> {
        let id = self.sessions.get(self.active)?.id;
        let group = self.sidebar.view.groups.group_of(id)?;
        let color = self.sidebar.view.groups.get(group)?.paint()?;
        let tokens = &store_in(&self.profile, cx).tokens;
        let color = ProjectColor::from_color(color.color(tokens));
        Some(folder_color::title_tint(&self.profile, color, cx))
    }

    /// Open in… and the splits at the right end of the title row, flush against each other with no
    /// gaps: the app's icon opens the folder, its chevron lists the others, and the two splits.
    /// Where the row is narrow they are one `⋯` menu.
    fn render_title_actions(&self, plan: &TitlePlan, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let folder = plan.facts.folder.clone();
        let menu = self.actions_menu(plan, folder.as_deref(), plan.fit.folded, cx);
        let anchor = Anchor::default();
        let open_menu = {
            let anchor = anchor.clone();
            move |window: &mut Window, cx: &mut App| {
                platform::show_menu(menu.clone(), anchor.below(), window, cx)
            }
        };
        if plan.fit.folded {
            return div()
                .id("title-more")
                .occlude()
                .relative()
                .flex_none()
                .child(anchor.probe())
                .child(
                    style::icon_button("title-more-menu", ui, IconName::Ellipsis, "Open in, split")
                        .in_title_row()
                        .on_click(move |_, window, cx| open_menu(window, cx)),
                )
                .into_any_element();
        }
        let split = |id: &'static str, icon: IconName, text: &str, action: ActionId, axis: Axis| {
            style::icon_button(id, ui, icon, keymap::with_key(text, action))
                .in_title_row()
                .on_click(cx.listener(move |this, _, window, cx| this.split_pane(axis, window, cx)))
                .into_any_element()
        };
        let open_in = folder.map(|folder| {
            let app = &plan.app.app;
            let tooltip = match &plan.app.missing {
                Some(missing) => format!("{missing} isn't installed. Open folder in {}", app.name),
                None => format!("Open folder in {}", app.name),
            };
            let bundle = app.bundle.clone();
            let icon = app_icon(ui, OpenIn::icon(cx, &app.bundle), &app.monogram, APP_ICON);
            [
                style::icon_button("open-in", ui, IconName::ExternalLink, tooltip)
                    .with_drawing(icon)
                    .in_title_row()
                    .on_click(move |_, _, _| open_in::open(&bundle, &folder))
                    .into_any_element(),
                div()
                    .id("open-in-chevron")
                    .occlude()
                    .relative()
                    .flex_none()
                    .child(anchor.probe())
                    .child(
                        style::icon_button("open-in-menu", ui, IconName::ChevronDown, "Open in another app")
                            .narrow(CHEVRON_W)
                            .on_click(move |_, window, cx| open_menu(window, cx)),
                    )
                    .into_any_element(),
            ]
        });
        let gap_after = open_in.is_some();
        div()
            .id("title-actions")
            .flex()
            .flex_none()
            .items_center()
            .children(open_in.into_iter().flatten())
            .when(gap_after, |d| d.child(div().flex_none().w(px(AFTER_CHEVRON))))
            .child(split("split-right", IconName::Columns2, "Split right", ActionId::SplitRight, Axis::Row))
            .child(split("split-down", IconName::Rows2, "Split down", ActionId::SplitDown, Axis::Column))
            .into_any_element()
    }

    /// The menu of Open in… (the chevron's) or, folded, of the whole group: "Open folder in" with
    /// every app that can (the one a click opens checked), and then the two splits.
    fn actions_menu(
        &self,
        plan: &TitlePlan,
        folder: Option<&Path>,
        with_splits: bool,
        cx: &mut Context<Self>,
    ) -> Menu {
        let apps: Vec<(Installed, Option<Icon>)> = folder
            .map(|_| OpenIn::apps(cx))
            .unwrap_or_default()
            .into_iter()
            .map(|app| {
                let icon = OpenIn::menu_icon(cx, &app);
                (app, icon)
            })
            .collect();
        let splits = with_splits.then(|| (self.can_split(Axis::Row), self.can_split(Axis::Column)));
        actions_menu(folder, &apps, &plan.app.app.bundle, splits, &cx.weak_entity())
    }

    /// Files dropped on the title bar, which covers the top of the terminal and so
    /// keeps the terminal itself from receiving them.
    fn drop_files_on_titlebar(&mut self, files: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        self.active_terminal().update(cx, |terminal, cx| terminal.drop_files(files, window, cx));
    }

    /// Gathers the row's plan for this frame (see [`TitlePlan`]).
    pub(super) fn title_plan(&self, window: &Window, cx: &mut Context<Self>) -> TitlePlan {
        let tint = self.title_tint(cx);
        self.plan_title(tint, window, cx)
    }
}

/// The actions menu: "Open folder in" `folder` with `apps` (the `chosen` bundle checked), then,
/// when `splits` says (whether the pane can split right and down), the two splits.
fn actions_menu(
    folder: Option<&Path>,
    apps: &[(Installed, Option<Icon>)],
    chosen: &str,
    splits: Option<(bool, bool)>,
    this: &WeakEntity<Workspace>,
) -> Menu {
    let menu = match folder {
        Some(folder) => apps.iter().fold(Menu::new().header("Open folder in"), |menu, (app, icon)| {
            let (bundle, folder) = (app.bundle.clone(), folder.to_path_buf());
            menu.item(
                Item::new(app.name.clone())
                    .icon(icon.clone())
                    .checked(app.bundle == chosen)
                    .on_select(move |_, _| open_in::open(&bundle, &folder)),
            )
        }),
        None => Menu::new(),
    };
    let Some((right, down)) = splits else { return menu };
    let split = |label: &'static str, action: ActionId, enabled: bool, axis: Axis| {
        Item::new(label)
            .key(keymap::key(action).and_then(KeyEquivalent::from_spec))
            .enabled(enabled)
            .on_select(platform::run_on(this, move |ws, window, cx| ws.split_pane(axis, window, cx)))
    };
    menu.separator().item(split("Split right", ActionId::SplitRight, right, Axis::Row)).item(split(
        "Split down",
        ActionId::SplitDown,
        down,
        Axis::Column,
    ))
}

/// An app's own icon, `size` points square, or its monogram in a rounded square where the icon
/// can't be read.
fn app_icon(ui: UiColors, icon: Option<Arc<Image>>, monogram: &str, size: f32) -> AnyElement {
    match icon {
        Some(image) => img(image).size(px(size)).flex_none().into_any_element(),
        None => style::agent_logo(None, SharedString::from(monogram.to_string()), size, ui.text),
    }
}

#[cfg(test)]
mod menu_model_tests {
    use std::path::Path;

    use gpui_kit::WeakEntity;
    use wormhole_core::platform::open_in::Installed;

    use super::actions_menu;

    fn app(bundle: &str, name: &str) -> (Installed, Option<crate::platform::Icon>) {
        (Installed { bundle: bundle.into(), name: name.into(), monogram: name[..1].into() }, None)
    }

    #[test]
    fn the_chosen_app_is_checked_and_the_splits_follow_when_folded() {
        let apps = [app("a.zed", "Zed"), app("b.code", "Code")];
        let this = WeakEntity::new_invalid();
        let folder = Some(Path::new("/tmp"));
        assert_eq!(
            actions_menu(folder, &apps, "b.code", Some((true, false)), &this).outline(),
            ["# Open folder in", "- Zed", "- Code [x]", "---", "- Split right", "- Split down [off]"]
        );
        assert_eq!(
            actions_menu(folder, &apps, "a.zed", None, &this).outline(),
            ["# Open folder in", "- Zed [x]", "- Code"],
            "the chevron's menu has no splits"
        );
        assert_eq!(
            actions_menu(None, &[], "", Some((true, true)), &this).outline(),
            ["- Split right", "- Split down"],
            "no folder, no apps, and no separator before the splits"
        );
    }
}
