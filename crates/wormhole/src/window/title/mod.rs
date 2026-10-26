//! The terminal's title row: the breadcrumb of where
//! the selected terminal is and what it does (`breadcrumb`), its extras (`extras`, `chips`), Open in…
//! and the two splits, laid out as the person arranged them (`row`, `parts`), the project's tint
//! behind them and the line under the row. A right-click on the row opens its menu of parts.
//!
//! It has its own row above the terminal's surface and never overlaps it. The row's height is [`style::TITLE_ROW_H`]; the line, when it is drawn, is the one
//! border width under it. An empty stretch of it is the window's title bar: dragging moves the
//! window and a double-click does what the system setting says (zoom, or minimize).

pub(in crate::window) mod breadcrumb;
pub(in crate::window) mod chips;
pub(in crate::window) mod extras;
pub(crate) mod parts;
pub(in crate::window) mod path_card;
pub(in crate::window) mod path_menu;
pub(in crate::window) mod row;
#[cfg(test)]
mod title_tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::platform::open_in::{Installed, Resolved};
use wormhole_core::project::ProjectColor;
use wormhole_core::theme::tokens::Color;
use wormhole_core::view::layout::title_parts::{PartKind, TitlePart, TitleRow};
use wormhole_core::view::layout::title_row::{PathStep, RowFit};
use wormhole_core::view::panes::Axis;
use wormhole_core::view::sidebar::GroupPaint;

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::menubar::model::KeyEquivalent;
use crate::open_in::{self, OpenIn};
use crate::platform::{self, Anchor, Icon, Item, Menu};
use crate::settings::{UiColors, store_in, ui_in};
use crate::style::TitleDrag as _;
use crate::style::{self, Edges, MenuMark};
use crate::terminal_view::Status;
use crate::theme_bridge::token;
use crate::window::Workspace;
use crate::window::title::extras::ExtraFacts;
use crate::window::title::row::End;

/// What the title row says about the terminal it is for, gathered once per frame.
#[derive(Debug)]
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
    /// The extras beside it (`window::title::extras`): its status, the agent's model and context, the ports
    /// and the pull request, each only if the row shows it and there is something to say.
    pub(super) extras: ExtraFacts,
    /// Which parts the row shows, and where (`title_row`).
    pub(super) row: TitleRow,
}

/// The project's colour as the row draws it.
#[cfg_attr(
    not(any(feature = "devctl", test, debug_assertions)),
    expect(
        dead_code,
        reason = "only devctl's title dump and the check on the chrome read the colour and its strength"
    )
)]
pub(super) struct Tint {
    pub(super) color: ProjectColor,
    /// How strong it is at the left edge: 36 %, less where the text would not read.
    pub(super) peak: f32,
    /// The line that takes its place under Increase Contrast.
    pub(super) line: Hsla,
}

/// Everything the title row draws, worked out once per frame: the terminal's facts, the parts at
/// each end (`row::ends`), what the narrow row gives up (core's `fit_row`), the app a click on Open
/// in… opens, and the project's tint.
pub(super) struct TitlePlan {
    pub(super) facts: Facts,
    pub(super) ends: (End, End),
    pub(super) steps: Vec<PathStep>,
    pub(super) fit: RowFit,
    pub(super) app: Resolved,
    pub(super) tint: Option<Tint>,
}

impl TitlePlan {
    /// Whether either end draws `part` (when the row has room for it).
    pub(super) fn ends_have(&self, part: TitlePart) -> bool {
        self.ends.0.parts.contains(&part) || self.ends.1.parts.contains(&part)
    }
}

/// The width of Open in… (the app's icon and its chevron), the two splits, and the one `⋯` they
/// fold into; each with the gap that separates it from the breadcrumb.
pub(super) const ROW_GAP: f32 = 4.0;
/// The title row is a bar this far in from the column's sides and the window's top, centred on the
/// traffic lights, with as much under it before the terminal's surface (under the whole title row,
/// [`style::TITLE_ROW_H`]: also where the terminals' tab bar starts when they are tabs).
pub(super) const BAR_INSET: f32 = 4.0;
pub(super) const OPEN_IN_W: f32 = style::HIT_MIN + CHEVRON_W + AFTER_CHEVRON;
const CHEVRON_W: f32 = 16.0;
/// After Open in…'s narrow chevron, so its glyph stands as far from the split's as the row's other
/// glyphs stand from each other (12 pt).
pub(super) const AFTER_CHEVRON: f32 = 4.0;
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
        let in_diff = self.covered(cx);
        let plan = (!in_diff).then(|| self.title_plan(window, cx));
        let (left_pad, right_pad) = self.title_row_pads(window);

        // The bar has no fill of its own: its content stands on the window. Under Increase Contrast
        // the group's colour is a line under it.
        let tint = plan.as_ref().and_then(|plan| plan.tint.as_ref());
        let line = tint.filter(|_| system.increase_contrast).map(|tint| tint.line);
        let ends = plan.as_ref().map(|plan| self.render_title_ends(plan, window, cx));
        let (left, right) = ends.unzip();
        let right_shrinks =
            plan.as_ref().is_some_and(|plan| plan.ends.1.parts.contains(&TitlePart::Terminal));
        // A right-click anywhere on the row opens the row's menu of parts, but on the folder, whose
        // own is the path menu. Laid over the row, so whatever lies over the row (the palette's scrim)
        // keeps the click. The diff's title has none.
        let menu = plan.as_ref().map(|plan| {
            let this = cx.weak_entity();
            let folder = plan.ends_have(TitlePart::Folder).then(|| self.path_anchor.clone());
            platform::on_right_click_except(folder, move |at, window, cx| {
                this.update(cx, |this, cx| this.open_title_menu(at, window, cx)).ok();
            })
        });
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
                        .title_drag()
                        .children(self.render_cover_title(cx))
                        // The left end gives way before the right one: the name in it is cut.
                        .children(left.map(|left| div().flex().min_w_0().flex_shrink(1.0).child(left)))
                        // The flexible space: the window's title bar.
                        .child(div().flex_1().min_w_0())
                        // The right end keeps its width, unless the name is there to give way.
                        .children(right.map(|right| {
                            div()
                                .flex()
                                .map(|d| if right_shrinks { d.min_w_0() } else { d.flex_none() })
                                .child(right)
                        })),
                    (left_pad, right_pad),
                ),
            )
            .children(menu);

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
            let (left, right) = inside(crate::window::chrome::sliding_title_pads(
                motion.edge,
                (left, right),
                motion.offset(t),
            ));
            d.pl(px(left)).pr(px(right))
        })
        .into_any_element()
    }

    /// What the selected terminal's group is painted with: all the window's chrome reads of the list's
    /// saved view, so the window hears of a change of the list only when this differs
    /// (`Regions::watch_sidebar`).
    pub(in crate::window) fn group_paint(&self, cx: &App) -> Option<GroupPaint> {
        let id = self.sessions.get(self.active)?.id;
        let groups = &self.sidebar_view(cx).groups;
        groups.get(groups.group_of(id)?)?.paint()
    }

    /// The tint behind the active terminal's title row: its group's colour, as the sidebar's card
    /// wears it. A terminal in no group (or in a group without a colour) has none. It only
    /// paints, so it needs no trust.
    pub(in crate::window) fn title_tint(&self, cx: &App) -> Option<Tint> {
        let color = self.group_paint(cx)?;
        let tokens = &store_in(&self.profile, cx).tokens;
        let color = ProjectColor::from_color(color.color(tokens));
        let store = store_in(&self.profile, cx);
        let bar = store.tokens.get("surface.window").map_or(Color::BLACK, |c| c.with_alpha(1.0));
        let text = store.tokens.get("text.primary").unwrap_or(Color::WHITE);
        Some(Tint {
            color,
            peak: color.tint_peak(bar, text),
            line: crate::theme_bridge::hsla(color.line(bar)),
        })
    }

    /// What the row's buttons draw for `part`: Open in… (the app's icon opens the folder, its chevron
    /// lists the others) or the two splits. Where the row is narrow both are one `⋯` menu, in the
    /// place of the first of them.
    pub(super) fn render_title_buttons(
        &self,
        plan: &TitlePlan,
        part: TitlePart,
        cx: &mut Context<Self>,
    ) -> Option<Vec<AnyElement>> {
        let ui = ui_in(&self.profile, cx);
        let open_in = plan.ends_have(TitlePart::OpenIn);
        let splits = plan.ends_have(TitlePart::Splits);
        // Open in… needs a folder (`row::has`), so its place in the ends says there is one.
        let folder = plan.facts.folder.clone().filter(|_| open_in);
        let anchor = Anchor::default();
        let menu = |with_splits: bool, cx: &mut Context<Self>| {
            let menu = self.actions_menu(plan, folder.as_deref(), with_splits, cx);
            let anchor = anchor.clone();
            // The chevron's mark follows the menu while it is open; the folded `⋯` has none.
            move |mark: Option<&MenuMark>, window: &mut Window, cx: &mut App| {
                let menu = match mark {
                    Some(mark) => mark.track(menu.clone(), cx),
                    None => menu.clone(),
                };
                platform::show_menu(menu, anchor.below(), window, cx)
            }
        };
        if plan.fit.folded {
            // The first of them in the row, left end first.
            let first =
                plan.ends.0.parts.iter().chain(&plan.ends.1.parts).find(|p| p.kind() == PartKind::Button);
            if first != Some(&part) {
                return None;
            }
            let open_menu = menu(splits, cx);
            let tip = text(match (open_in, splits) {
                (true, true) => TITLE_ROW_MORE_OPEN_IN_SPLIT,
                (true, false) => TITLE_ROW_MORE_OPEN_IN,
                _ => TITLE_ROW_MORE_SPLIT,
            });
            return Some(vec![
                div()
                    .id("title-more")
                    .occlude()
                    .relative()
                    .flex_none()
                    .child(anchor.probe())
                    .child(
                        style::icon_button("title-more-menu", ui, IconName::Ellipsis, tip)
                            .in_title_row()
                            .on_click(move |_, window, cx| open_menu(None, window, cx)),
                    )
                    .into_any_element(),
            ]);
        }
        if part == TitlePart::Splits {
            let split = |id: &'static str,
                         icon: IconName,
                         half: style::Half,
                         text: &str,
                         action: ActionId,
                         axis: Axis| {
                style::icon_button(id, ui, icon, keymap::with_key(text, action))
                    .in_title_row()
                    .motion(style::IconMotion::NewPane(half))
                    .on_click(cx.listener(move |this, _, window, cx| this.split_pane(axis, window, cx)))
                    .into_any_element()
            };
            return Some(vec![
                split(
                    "split-right",
                    IconName::Columns2,
                    style::Half::Right,
                    "Split right",
                    ActionId::SplitRight,
                    Axis::Row,
                ),
                split(
                    "split-down",
                    IconName::Rows2,
                    style::Half::Below,
                    "Split down",
                    ActionId::SplitDown,
                    Axis::Column,
                ),
            ]);
        }
        let open_menu = menu(false, cx);
        let folder = folder?;
        let app = &plan.app.app;
        let tooltip = match &plan.app.missing {
            Some(missing) => format!("{missing} isn't installed. Open folder in {}", app.name),
            None => format!("Open folder in {}", app.name),
        };
        let bundle = app.bundle.clone();
        let icon = app_icon(ui, OpenIn::icon(cx, &app.bundle), &app.monogram, APP_ICON);
        Some(vec![
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
                        .menu_chevron(move |mark, window, cx| open_menu(Some(mark), window, cx)),
                )
                .into_any_element(),
        ])
    }

    /// The menu of Open in… (the chevron's) or, folded, of the whole group: "Open folder in" with
    /// every app that can (the one a click opens checked), and then the two splits. Without a
    /// `folder` (no Open in… in the row) it is the splits alone.
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
