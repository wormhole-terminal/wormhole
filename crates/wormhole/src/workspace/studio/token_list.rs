//! The token list: a filter field and All / Changed / Low contrast over
//! the tokens in folder bands, read like the sidebar: a swatch or shape at x 16, the name in
//! the code font at x 40, the value faint on the right.

use std::collections::BTreeSet;

use super::draft::{Show, group_size, in_area, visible};
use super::*;
use style::{BandKind, BandParts, Section};

/// The token list's width.
pub(super) const LIST_W: f32 = 300.0;

/// The longest value a row shows before it is cut short (an expression can be long).
const VALUE_MAX_W: f32 = 112.0;

impl StudioWindow {
    pub(super) fn render_token_list(&self, panel: Hsla, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let tokens = tokens_of(cx);
        let (code_font, _) = crate::fonts::code_font(cx);
        let low = self.low_contrast_tokens(&tokens);
        let changed = |name: &str| self.is_changed(name);
        let query = self.filter.read(cx).value().to_string();
        // The gallery's area filter narrows the list too: what that part's components draw with.
        let area = self.area.part();
        let shown = visible(self.show, area, &query, &changed, &|name| low.contains(name));
        // Filtering shows what it found: every group that has a hit, open.
        let filtering = self.show != Show::All || area.is_some() || !query.trim().is_empty();
        let changed_count = visible(Show::Changed, area, "", &changed, &|_| false)
            .iter()
            .map(|(_, t)| t.len())
            .sum::<usize>();
        let low_count = low.iter().filter(|name| in_area(name, area)).count();

        let show = |id: &'static str, label: String, value: Show| {
            style::segment(id, ui, label, self.show == value)
                .on_click(cx.listener(move |this, _, _, cx| this.set_show(value, cx)))
        };
        let header = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .child(
                Input::new(&self.filter)
                    .small()
                    .cleanable(true)
                    .prefix(style::icon(IconName::Search).text_color(ui.muted)),
            )
            .child(
                style::segmented(ui)
                    .child(show("studio-show-all", "All".into(), Show::All))
                    .child(show("studio-show-changed", format!("Changed · {changed_count}"), Show::Changed))
                    .child(show("studio-show-low", format!("Low contrast · {low_count}"), Show::LowContrast)),
            );

        let mut list = div().flex().flex_col().pb_4();
        // The token being edited stays in the editor when the area leaves it out: say why the
        // list doesn't show it rather than drop it silently.
        if let Some(name) = self.selected.as_deref().filter(|name| !in_area(name, area)) {
            let note = format!("{name} isn\u{2019}t used in {}.", self.area.label());
            list = list.child(
                div()
                    .id("studio-not-in-area")
                    .px_4()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_caption()
                    .text_color(ui.muted)
                    .child(style::icon(IconName::Info).text_color(ui.faint))
                    .child(div().min_w_0().child(note)),
            );
        }
        let mut joined = false;
        for (group, names) in shown {
            let open = filtering || self.open_groups.contains(group.name);
            let group_changed = names.iter().any(|name| changed(name));
            let group_low = names.iter().filter(|name| low.contains(**name)).count();
            let name = group.name;
            let marks = div()
                .flex_1()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .when(group_low > 0, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_caption()
                            .text_color(ui.warning_text)
                            .child(style::icon(IconName::TriangleAlert).text_color(ui.warning))
                            .child(group_low.to_string()),
                    )
                })
                .when(group_changed, |d| d.child(style::changed_dot(ui)));
            list = list.child(style::band(
                SharedString::from(format!("studio-group-{name}")),
                ui,
                BandParts {
                    kind: BandKind::Section,
                    section: if open { Section::Open } else { Section::Collapsed },
                    title: name.into(),
                    detail: Some(marks.into_any_element()),
                    // The group's size in the area, also while a filter lists fewer of it.
                    count: Some(group_size(group, area).to_string().into()),
                    add: None,
                    on_toggle: (!filtering).then(|| -> style::ClickHandler {
                        let this = cx.weak_entity();
                        Box::new(move |_, _, cx| {
                            let _ = this.update(cx, |studio, cx| studio.toggle_group(name, cx));
                        })
                    }),
                    drop_target: false,
                    sticky: false,
                    joined,
                },
            ));
            // Two folded bands in a row share one hairline, as in the sidebar.
            joined = !open;
            if !open {
                continue;
            }
            for name in names {
                list = list.child(self.token_row(name, &tokens, &low, code_font.clone(), ui, cx));
            }
        }

        div()
            .id("studio-token-list")
            .w(px(LIST_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .rule_right(ui)
            .bg(panel)
            .child(header)
            .child(
                div()
                    .id("studio-token-rows")
                    .track_scroll(&self.list_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(list),
            )
            .into_any_element()
    }

    fn token_row(
        &self,
        name: &str,
        tokens: &ThemeTokens,
        low: &BTreeSet<String>,
        code_font: SharedString,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.selected.as_deref() == Some(name);
        let (preview, value) = match tokens.metrics.get(name).filter(|_| is_metric(name)) {
            Some(v) => (metric_chip(ui, name, v), format!("{}px", number(v))),
            None => {
                let color = tokens.get(name).unwrap_or(crate::theme_bridge::MISSING_TOKEN);
                (color_chip(ui, color), color.to_hex())
            }
        };
        // What the draft says itself, as typed; otherwise the value it resolves to.
        let value = override_of(&self.draft, name).filter(|_| !is_metric(name)).unwrap_or(value);
        let name_owned = name.to_string();
        style::flush_row(SharedString::from(format!("token-{name}")), ui, selected)
            .h(px(style::ROW_H))
            .on_click(cx.listener(move |this, _, window, cx| this.select_token(&name_owned, window, cx)))
            .child(style::lead(preview))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_caption()
                            .font_family(code_font.clone())
                            .child(SharedString::from(name.to_string())),
                    )
                    .when(low.contains(name), |d| {
                        d.child(style::icon(IconName::TriangleAlert).text_color(ui.warning))
                    })
                    // On the selected row's tint, the dot takes the tint's text color.
                    .when(self.is_changed(name), |d| {
                        d.child(style::changed_dot(ui).when(selected, |d| d.bg(ui.selected_text)))
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .max_w(px(VALUE_MAX_W))
                    .truncate()
                    .text_caption()
                    .font_family(code_font)
                    .text_color(if selected { ui.selected_muted } else { ui.faint })
                    .child(value),
            )
            .into_any_element()
    }

    /// Whether the draft sets `name` differently from how it started.
    pub(super) fn is_changed(&self, name: &str) -> bool {
        override_of(&self.draft, name) != override_of(&self.initial, name)
    }

    fn set_show(&mut self, show: Show, cx: &mut Context<Self>) {
        self.show = show;
        cx.notify();
    }

    fn toggle_group(&mut self, name: &'static str, cx: &mut Context<Self>) {
        if !self.open_groups.remove(name) {
            self.open_groups.insert(name);
        }
        cx.notify();
    }
}
