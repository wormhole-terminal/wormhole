//! The gallery as a view of its own, so that scrolling or animating it redraws it alone: a wheel
//! tick (or an animation's frame) notifies the view it happens in, and that used to be the whole
//! Studio window. The page is what it always was, two columns running down on their own and the
//! wide specimens under them, in one scrolling div; what changed is that a piece far from the
//! viewport is a placeholder of the height it had when it was last built (`layout`).

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::*;
use wormhole_core::theme::tokens::{TokenUse, uses_of};

use super::layout::{Columns, Measures, Piece, columns, extents, meeting};
use super::syntax::CodeSample;
use super::{Area, Env, SPECIMENS, TWO_COLUMNS_MIN_W, section};
use crate::settings::{SettingsStore, UiColors};
use crate::style::{self, TextRole};
use crate::workspace::studio::token_editor::EDITOR_W;
use crate::workspace::studio::token_list::LIST_W;
use crate::workspace::studio::{StudioWindow, t, tokens_of, ui};

/// How far above and below the viewport pieces are built, in viewports: a wheel tick or two
/// doesn't show a placeholder.
const OVERDRAW: f32 = 0.25;
/// The viewport's height until the page has been drawn once.
const VIEWPORT_H: f32 = 900.0;

pub(in crate::workspace::studio) struct GalleryView {
    studio: WeakEntity<StudioWindow>,
    /// The token being edited and the part of the app shown: the window's, as of the last draw.
    pub(super) selected: Option<String>,
    area: Area,
    /// What `columns` and the measures are of: the area, whether the page has two columns, and
    /// its width (what the heights depend on).
    page: Option<(Area, bool, i32)>,
    columns: Columns,
    measures: Rc<RefCell<Measures>>,
    scroll: ScrollHandle,
    /// The gallery's text field and the item editor's, shown by the specimens that have one.
    pub(super) sample_input: Entity<InputState>,
    pub(super) item_editor: Entity<InputState>,
    /// The syntax sample, parsed once.
    pub(super) sample: RefCell<Option<CodeSample>>,
    /// devctl `studio-scroll`: where to scroll to, and how many frames it may take until the page
    /// is long enough to reach it (pieces not built yet are only guessed at).
    #[cfg(feature = "devctl")]
    scroll_to: Option<(f32, u32)>,
    /// How often the view was drawn and how many specimens' pieces it built (the counting tests).
    #[cfg(test)]
    pub(in crate::workspace::studio) counts: Counts,
    _subscriptions: Vec<Subscription>,
}

#[cfg(test)]
#[derive(Clone, Debug, Default)]
pub(in crate::workspace::studio) struct Counts {
    pub(in crate::workspace::studio) renders: usize,
    /// The pieces built (a cell, as the build is through `&self`).
    pub(in crate::workspace::studio) pieces: std::cell::Cell<usize>,
}

impl GalleryView {
    pub(in crate::workspace::studio) fn new(
        studio: &Entity<StudioWindow>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            studio: studio.downgrade(),
            selected: None,
            area: Area::All,
            page: None,
            columns: Columns::default(),
            measures: Rc::default(),
            scroll: ScrollHandle::new(),
            sample_input: cx.new(|cx| InputState::new(window, cx).placeholder("Search themes")),
            item_editor: cx.new(|cx| InputState::new(window, cx).default_value("Rename this task")),
            sample: RefCell::new(None),
            #[cfg(feature = "devctl")]
            scroll_to: None,
            #[cfg(test)]
            counts: Counts::default(),
            _subscriptions: vec![
                // The window says when the selection, the area or the draft changes.
                cx.observe(studio, |_, _, cx| cx.notify()),
                // A theme that is switched to, or previewed, changes every color on the page.
                cx.observe_global::<SettingsStore>(|_, cx| cx.notify()),
            ],
        }
    }

    /// Select a token, as the window's own list does.
    pub(super) fn select_token(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.studio.update(cx, |studio, cx| studio.select_token(name, window, cx)).ok();
    }

    /// A probe: reports the bounds it is laid out at, fills the element it is put in. A page that
    /// moves under it (a piece measured, a column longer than guessed) is drawn again.
    fn probe(&self, report: impl FnOnce(&mut Measures, Bounds<Pixels>) -> bool + 'static) -> Canvas<()> {
        let measures = self.measures.clone();
        canvas(
            move |bounds, window, _| {
                let moved = report(&mut measures.borrow_mut(), bounds);
                if moved {
                    window.request_animation_frame();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }

    /// One piece of a specimen: built if `near` the viewport, else a space as tall as it is.
    fn piece(&self, piece: Piece, near: bool, env: &Env, cx: &mut Context<Self>) -> AnyElement {
        if !near {
            return div().w_full().h(px(self.measures.borrow().height(piece))).into_any_element();
        }
        #[cfg(test)]
        self.counts.pieces.set(self.counts.pieces.get() + 1);
        let content = (SPECIMENS[piece.specimen].render)(self, env, piece.part, cx);
        div()
            .relative()
            .child(content)
            .child(self.probe(move |m, bounds| m.height_of(piece, f32::from(bounds.size.height))))
            .into_any_element()
    }

    /// A column: its specimens under their labels, with the pieces `near` the viewport built.
    fn stack(&self, pieces: &[Piece], near: &[bool], env: &Env, cx: &mut Context<Self>) -> Div {
        let mut column = div().flex_1().min_w_0().flex().flex_col().gap_6();
        let mut at = 0;
        while at < pieces.len() {
            let specimen = &SPECIMENS[pieces[at].specimen];
            let end = at + pieces[at..].iter().take_while(|p| p.specimen == pieces[at].specimen).count();
            let parts = div()
                .flex()
                .flex_col()
                .gap(px(specimen.part_gap))
                .children((at..end).map(|ix| self.piece(pieces[ix], near[ix], env, cx)));
            column =
                column.child(section(env.ui, specimen.title, specimen.description, parts.into_any_element()));
            at = end;
        }
        column
    }

    /// The filter, and how many things the selected token is drawn on.
    fn header(&self, uses: &[&TokenUse], ui: UiColors) -> AnyElement {
        let mut filter = style::segmented(ui);
        for area in Area::ALL {
            let studio = self.studio.clone();
            filter = filter.child(
                style::segment(
                    SharedString::from(format!("studio-area-{}", area.label())),
                    ui,
                    area.label(),
                    self.area == area,
                )
                .on_click(move |_, _, cx| {
                    studio.update(cx, |studio, cx| studio.set_area(area, cx)).ok();
                }),
            );
        }
        let count = self.selected.as_ref().map(|name| {
            let text = match uses.len() {
                0 => format!("Nothing draws {name} directly"),
                1 => format!("1 use of {name}"),
                n => format!("{n} uses of {name}"),
            };
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1()
                .text_caption()
                .text_color(if uses.is_empty() { ui.faint } else { ui.link })
                .child(style::icon(IconName::Target))
                .child(div().min_w_0().truncate().child(text))
        });
        div().flex().items_center().justify_between().gap_4().child(filter).children(count).into_any_element()
    }

    /// devctl `studio-scroll <y>`: the page scrolled to `y` pixels from its top.
    #[cfg(feature = "devctl")]
    pub(in crate::workspace::studio) fn devctl_scroll(&mut self, y: f32, cx: &mut Context<Self>) {
        self.scroll_to = Some((y.max(0.0), 30));
        cx.notify();
    }

    /// One wheel tick of `dy` pixels (`studio-frames scroll`, and the tests').
    #[cfg(any(test, feature = "devctl"))]
    pub(in crate::workspace::studio) fn scroll_by(&mut self, dy: f32, cx: &mut Context<Self>) {
        let at = self.scroll.offset();
        self.scroll.set_offset(point(at.x, at.y - px(dy)));
        cx.notify();
    }
}

impl Render for GalleryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        {
            self.counts.renders += 1;
        }
        let (area, selected) =
            self.studio.read_with(cx, |studio, _| (studio.area, studio.selected.clone())).unwrap_or_default();
        self.area = area;
        self.selected = selected;
        let width = f32::from(window.viewport_size().width) - LIST_W - EDITOR_W;
        let page = (area, width >= TWO_COLUMNS_MIN_W, width.round() as i32);
        if self.page != Some(page) {
            self.page = Some(page);
            self.columns = columns(page.0, page.1);
            self.measures.borrow_mut().reset();
        }
        #[cfg(feature = "devctl")]
        if let Some((y, frames)) = self.scroll_to {
            self.scroll.set_offset(point(px(0.0), px(-y)));
            let reached = f32::from(self.scroll.max_offset().y) >= y - 0.5;
            self.scroll_to = (!reached && frames > 0).then_some((y, frames - 1));
            if self.scroll_to.is_some() {
                window.request_animation_frame();
            }
        }

        let ui = ui(cx);
        let tokens = tokens_of(cx);
        let uses = self.selected.as_deref().map(uses_of).unwrap_or_default();
        let env = Env { ui, tokens: &tokens, uses: uses.iter().map(|u| u.id).collect() };

        // The viewport and a stretch of overdraw on either side, in points down the page; the
        // pieces that meet it are built.
        let viewport = match f32::from(self.scroll.bounds().size.height) {
            h if h > 0.0 => h,
            _ => VIEWPORT_H,
        };
        let top = -f32::from(self.scroll.offset().y);
        let window_of = (top - viewport * OVERDRAW)..(top + viewport * (1.0 + OVERDRAW));
        let (left_near, right_near, wide_near) = {
            let m = self.measures.borrow();
            let height = |piece| m.height(piece);
            let (left, left_end) = extents(&self.columns.left, m.columns_top(), height);
            let (right, right_end) = extents(&self.columns.right, m.columns_top(), height);
            let (wide, _) = extents(&self.columns.wide, m.wide_top(left_end.max(right_end)), height);
            (meeting(&left, &window_of), meeting(&right, &window_of), meeting(&wide, &window_of))
        };

        let columns = self.columns.clone();
        let two_columns = div()
            .relative()
            .flex()
            .items_start()
            .gap_6()
            .child(self.probe(|m, bounds| m.block_top(0, f32::from(bounds.origin.y) - m.origin)))
            .child(self.stack(&columns.left, &left_near, &env, cx))
            .child(self.stack(&columns.right, &right_near, &env, cx));
        let wide = self
            .stack(&columns.wide, &wide_near, &env, cx)
            .relative()
            .child(self.probe(|m, bounds| m.block_top(1, f32::from(bounds.origin.y) - m.origin)));
        div()
            .id("studio-gallery")
            .track_scroll(&self.scroll)
            .size_full()
            .min_w_0()
            .overflow_y_scroll()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .text_color(t(&tokens, "text.primary"))
            .child(
                canvas(
                    {
                        let measures = self.measures.clone();
                        move |bounds, _, _| measures.borrow_mut().origin = f32::from(bounds.origin.y)
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(self.header(&uses, ui))
            .child(two_columns)
            .child(wide)
    }
}
