//! The terminal's title row: the breadcrumb of where
//! the selected terminal is and what it does (`breadcrumb`), Open in… and the two splits flush at
//! the right end, the project's tint behind them and the line under the row.
//!
//! It floats over the top of the terminal, so earlier output scrolls under it, blurred by the
//! terminal. The row's height is [`style::TITLE_ROW_H`]; the line, when it is drawn, is the one
//! border width under it. An empty stretch of it is the window's title bar: dragging moves the
//! window and a double-click does what the system setting says (zoom, or minimize).

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use wormhole_core::agents::AgentId;
use wormhole_core::layout::title_row::{PathStep, RowFit};
use wormhole_core::open_in::{Installed, Resolved};
use wormhole_core::panes::Axis;
use wormhole_core::project::ProjectColor;

use super::chrome::TitleDrag as _;
use super::focus::MenuBuilder;
use super::folder_color;
use super::open_in::{self, OpenIn};
use super::title_extras::ExtraFacts;
use super::*;
use crate::theme_bridge::{self, token};

/// What the title row says about the terminal it is for, gathered once per frame.
pub(super) struct Facts {
    pub(super) id: i64,
    /// Where the terminal is, and how that reads (`~/Projects/tube`).
    pub(super) folder: Option<PathBuf>,
    pub(super) shown: Option<String>,
    pub(super) status: Status,
    pub(super) agent: Option<AgentId>,
    /// The terminal's name: its title, else what runs in it.
    pub(super) label: SharedString,
    /// How it is doing after the name ("working · 4m"); nothing while it is idle.
    pub(super) words: Option<String>,
    /// The branch of the folder: `Some(None)` is a detached HEAD.
    pub(super) branch: Option<Option<String>>,
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
    /// Where the folder starts, from the left edge of the terminal's column.
    pub(super) path_x: f32,
}

/// The width of Open in… (the app's icon and its chevron), the two splits, and the one `⋯` they
/// fold into; each with the gap that separates it from the breadcrumb.
pub(super) const ROW_GAP: f32 = 4.0;
pub(super) const OPEN_IN_W: f32 = style::HIT_MIN + CHEVRON_W;
const CHEVRON_W: f32 = 16.0;
pub(super) const SPLITS_W: f32 = 2.0 * style::HIT_MIN;
pub(super) const MORE_W: f32 = style::HIT_MIN;
/// The app's icon in the Open in… button, and in its menu.
const APP_ICON: f32 = style::TITLE_GLYPH;

impl Workspace {
    /// The terminal's title row: the selected terminal's breadcrumb (or the diff's title) and the
    /// buttons at the right end, floating over the top of the terminal.
    pub(super) fn render_terminal_title_row(
        &self,
        scrolled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let (drop_tint, fill, system, border) = {
            let store = cx.global::<SettingsStore>();
            let system = store.system();
            // Under Reduce Transparency and Increase Contrast the bar is solid, with no
            // see-through to the blurred text.
            let fill = Hsla { a: if system.opaque() { 1.0 } else { 0.62 }, ..store.palette.background };
            (token(&store.tokens, "state.highlight"), fill, system, store.settings.title_border)
        };
        let in_diff = self.diff_view.is_some();
        let plan = (!in_diff).then(|| self.title_plan(window, cx));
        let (left, right) = self.title_row_pads(window);

        // The tint is a gradient from the left; under Increase Contrast it gives way to a line.
        let tint = plan.as_ref().and_then(|plan| plan.tint.as_ref());
        let layers = tint
            .filter(|_| !system.increase_contrast)
            .map(|tint| style::tint_layers(theme_bridge::hsla(tint.color.color()), tint.peak))
            .unwrap_or_default();
        let line = tint.filter(|_| system.increase_contrast).map(|tint| tint.line);
        let strip = plan.as_ref().map(|plan| self.render_title_strip(plan, window, cx));
        let extras = plan.as_ref().and_then(|plan| self.render_title_extras(plan, cx));
        let actions = plan.as_ref().map(|plan| self.render_title_actions(plan, cx));
        let card = plan.as_ref().and_then(|plan| self.render_path_card(plan, self.title_column(), cx));

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
            // The line under the bar as the settings say; no shadow: the blur is the cue. A project's
            // colour as a line stays whatever they say, since it is the project's marker.
            .map(|d| match line {
                Some(color) => d.rule_bottom_in(ui, color, 2.0),
                None if border.shows(scrolled) => d.rule_bottom(ui),
                None => d,
            })
            // A tint only when content is scrolled under the bar; otherwise the bar blends into
            // the terminal, like Finder.
            .when(scrolled, |d| d.bg(fill))
            // The diff view replaces the terminal, so there is nothing to drop onto then.
            .when(!in_diff, |d| {
                d.drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(drop_tint)).on_drop(cx.listener(
                    |this, files: &ExternalPaths, window, cx| this.drop_files_on_titlebar(files, window, cx),
                ))
            })
            .children(layers)
            .child(
                self.with_title_pads(
                    div()
                        .h(px(style::TITLE_ROW_H))
                        .flex_none()
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
            )
            .children(card)
            .into_any_element()
    }

    /// The title row's padding: `pads` at rest, and following the column's edge while one slides
    /// (`chrome::sliding_title_pads`), so the breadcrumb and the buttons move with it instead of
    /// jumping when it lands.
    fn with_title_pads(&self, row: Div, (left, right): (f32, f32)) -> AnyElement {
        let Some(motion) = self.slide_motion() else {
            return row.pl(px(left)).pr(px(right)).into_any_element();
        };
        row.with_animation(motion.id("slide-title"), motion.animation(), move |d, t| {
            let (left, right) =
                super::chrome::sliding_title_pads(motion.edge, (left, right), motion.offset(t));
            d.pl(px(left)).pr(px(right))
        })
        .into_any_element()
    }

    /// The tint behind the active terminal's title row, if any: the colour its folder wears (the
    /// person's own, for every window), else the colour its project names while the project's
    /// Settings switch is on. It only paints, so it needs no trust.
    fn title_tint(&self, cx: &App) -> Option<Tint> {
        let color = self.folder_tint_color(cx).or_else(|| self.project_color(cx))?;
        Some(folder_color::title_tint(color, cx))
    }

    /// Open in… and the splits at the right end of the title row, flush against each other with no
    /// gaps: the app's icon opens the folder, its chevron lists the others, and the two splits.
    /// Where the row is narrow they are one `⋯` menu.
    fn render_title_actions(&self, plan: &TitlePlan, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let folder = plan.facts.folder.clone();
        let menu = self.actions_menu(plan, folder.as_deref(), plan.fit.folded, cx);
        if plan.fit.folded {
            return div()
                .id("title-more")
                .occlude()
                .flex_none()
                .child(
                    style::icon_button("title-more-menu", ui, IconName::Ellipsis, "Open in, split")
                        .in_title_row()
                        .menu_trigger()
                        .dropdown_menu_with_anchor(Anchor::TopRight, move |m, w, c| menu(m, w, c)),
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
                    .flex_none()
                    .child(
                        style::icon_button("open-in-menu", ui, IconName::ChevronDown, "Open in another app")
                            .narrow(CHEVRON_W)
                            .menu_trigger()
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |m, w, c| menu(m, w, c)),
                    )
                    .into_any_element(),
            ]
        });
        div()
            .id("title-actions")
            .flex()
            .flex_none()
            .items_center()
            .children(open_in.into_iter().flatten())
            .child(split("split-right", IconName::Columns2, "Split right", ActionId::SplitRight, Axis::Row))
            .child(split("split-down", IconName::Rows2, "Split down", ActionId::SplitDown, Axis::Column))
            .into_any_element()
    }

    /// What builds the menu of Open in… (the chevron's) or, folded, of the whole group: "Open folder
    /// in" with every app that can (the one a click opens checked), and then the two splits.
    fn actions_menu(
        &self,
        plan: &TitlePlan,
        folder: Option<&Path>,
        with_splits: bool,
        cx: &mut Context<Self>,
    ) -> MenuBuilder {
        let ui = ui(cx);
        let this = cx.weak_entity();
        let folder = folder.map(Path::to_path_buf);
        let chosen = plan.app.app.bundle.clone();
        let apps: Vec<(Installed, Option<Arc<Image>>)> = folder
            .as_ref()
            .map(|_| OpenIn::apps(cx))
            .unwrap_or_default()
            .into_iter()
            .map(|app| {
                let icon = OpenIn::icon(cx, &app.bundle);
                (app, icon)
            })
            .collect();
        let (right, down) = (self.can_split(Axis::Row), self.can_split(Axis::Column));
        Rc::new(move |mut menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| {
            if let Some(folder) = &folder {
                menu = menu.label("Open folder in");
                for (app, icon) in &apps {
                    let (bundle, folder) = (app.bundle.clone(), folder.clone());
                    let (name, monogram, icon) = (app.name.clone(), app.monogram.clone(), icon.clone());
                    menu = menu.item(
                        PopupMenuItem::element(move |_, _| {
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(app_icon(ui, icon.clone(), &monogram, APP_ICON))
                                .child(name.clone())
                        })
                        .checked(app.bundle == chosen)
                        .on_click(move |_, _, _| open_in::open(&bundle, &folder)),
                    );
                }
            }
            if with_splits {
                if folder.is_some() {
                    menu = menu.separator();
                }
                let item = |label: &'static str, action: Box<dyn Action>, enabled: bool, axis: Axis| {
                    let this = this.clone();
                    PopupMenuItem::new(label).action(action).disabled(!enabled).on_click(
                        move |_, window, cx| {
                            this.update(cx, |ws, cx| ws.split_pane(axis, window, cx)).ok();
                        },
                    )
                };
                menu = menu
                    .item(item("Split right", Box::new(actions::SplitRight), right, Axis::Row))
                    .item(item("Split down", Box::new(actions::SplitDown), down, Axis::Column));
            }
            menu
        })
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

/// An app's own icon, `size` points square, or its monogram in a rounded square where the icon
/// can't be read.
fn app_icon(ui: UiColors, icon: Option<Arc<Image>>, monogram: &str, size: f32) -> AnyElement {
    match icon {
        Some(image) => img(image).size(px(size)).flex_none().into_any_element(),
        None => style::agent_logo(None, SharedString::from(monogram.to_string()), size, ui.text),
    }
}
