//! `SpecimenMock`, generated from the design's markup. Do not edit it by hand: it is rewritten whenever the design changes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme_bridge::token;
use crate::ui::{Handler, Look, Rule, Sides};

/// One item of `tabs`.
#[derive(Default)]
pub struct Tab {
    pub name: SharedString,
    pub selected: bool,
    pub pick: Option<Handler>,
}

/// One item of `facts`.
#[derive(Default)]
pub struct Fact {
    pub text: SharedString,
}

/// What `SpecimenMock` is drawn from.
pub struct SpecimenMockProps {
    pub id: ElementId,
    pub spoken: SharedString,
    pub tabs_label: SharedString,
    pub hint: SharedString,
    pub sample: AnyElement,
    pub tabs: Vec<Tab>,
    pub facts: Vec<Fact>,
}

impl SpecimenMockProps {
    /// The element tree, in the colours and the code font of `look`.
    pub fn render(self, look: &Look) -> Stateful<Div> {
        let t_border = token(look.tokens, "border");
        let t_button_hover = token(look.tokens, "button.hover");
        let t_input_background = token(look.tokens, "input.background");
        let t_surface_panel = token(look.tokens, "surface.panel");
        let t_text_faint = token(look.tokens, "text.faint");
        let t_text_muted = token(look.tokens, "text.muted");
        let t_text_primary = token(look.tokens, "text.primary");
        let borders = look.borders;
        let Self { id, spoken, tabs_label, hint, sample, tabs, facts } = self;

        div()
            .id(id)
            .role(Role::Group)
            .aria_label(spoken.clone())
            .flex()
            .flex_col()
            .bg(t_surface_panel)
            .child(div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .p(px(8.0))
                .rule(borders, Sides::BOTTOM, 1.0, t_border, false)
                .child(div()
                    .id("specimen-tabs")
                    .role(Role::Group)
                    .aria_label(tabs_label.clone())
                    .flex()
                    .p(px(2.0))
                    .rounded(px(7.0))
                    .bg(t_input_background)
                    .children(tabs.into_iter().enumerate().flat_map(|(position, t)| {
                        let t_selected = t.selected;
                        let mut out: Vec<AnyElement> = Vec::new();
                        out.push(
                            div()
                                .id(("e1", position))
                                .role(Role::Button)
                                .aria_selected(t_selected)
                                .focusable().tab_stop(true)
                                .flex()
                                .items_center()
                                .h(px(22.0))
                                .py(px(0.0)).px(px(10.0))
                                .rounded(px(5.0))
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(t_text_muted)
                                .cursor_pointer()
                                .when(t_selected, |this| {
                                    this
                                        .bg(t_button_hover)
                                        .text_color(t_text_primary)
                                })
                                .when_some(t.pick.clone(), |this, handler| {
                                    this.on_click(move |event, window, cx| handler(event, window, cx))
                                })
                                .child(t.name.clone())
                                .into_any_element(),
                        );
                        out
                    })))
                .child(div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(rems(0.75))
                    .line_height(rems(1.0))
                    .text_color(t_text_faint)
                    .child(hint.clone())))
            .child(sample)
            .child(div()
                .flex()
                .flex_wrap()
                .gap_y(px(4.0)).gap_x(px(16.0))
                .py(px(8.0)).px(px(16.0))
                .rule(borders, Sides::TOP, 1.0, t_border, false)
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .text_color(t_text_muted)
                .children(facts.into_iter().flat_map(|f| {
                    let mut out: Vec<AnyElement> = Vec::new();
                    out.push(
                        div()
                            .child(f.text.clone())
                            .into_any_element(),
                    );
                    out
                })))
    }
}
