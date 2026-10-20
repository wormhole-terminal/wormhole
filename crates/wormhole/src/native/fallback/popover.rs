//! A [`Popover`] as a card GPUI draws over the window, as the folder's and the group's cards were
//! before they were native: for platforms without a native popover, and for a window with no native
//! view to open one in. A layer the workspace renders holds the open one; its closures run from
//! GPUI's own handlers, so there is nothing to queue here (see `popover` for why the native one
//! must). The colour of one's own needs the system's colour panel, which only macOS has here: the
//! card shows its nine dots alone.

use std::collections::HashMap;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::native::popover::{Behavior, Glyph, Handle, Popover, Row};
use crate::settings::ui;
use crate::style::{self, TextRole as _};

/// The card's width, with and without a path in it.
const WIDTH: f32 = 276.0;
const WIDTH_PATH: f32 = 440.0;
/// The gap between the element and the card.
const GAP: f32 = 4.0;

/// The open popover of one window, and the element that draws it.
#[derive(Default)]
pub struct Layer {
    open: Option<Open>,
}

struct Open {
    popover: Popover,
    anchor: Bounds<Pixels>,
    field: Option<Entity<InputState>>,
    /// What a button says now ("Copied") instead of its title, by its place among the buttons.
    says: HashMap<usize, SharedString>,
    focus: FocusHandle,
    _events: Vec<Subscription>,
}

/// The layers of the windows that have had one.
#[derive(Default)]
struct Layers(HashMap<WindowId, Entity<Layer>>);

impl Global for Layers {}

/// `window`'s layer: the workspace puts it in its tree, [`show`] opens popovers in it.
pub fn layer(window: &Window, cx: &mut App) -> Entity<Layer> {
    let id = window.window_handle().window_id();
    if let Some(layer) = cx.try_global::<Layers>().and_then(|layers| layers.0.get(&id)) {
        return layer.clone();
    }
    let layer = cx.new(|_| Layer::default());
    let open: Vec<WindowId> = cx.windows().iter().map(|handle| handle.window_id()).collect();
    let layers = cx.default_global::<Layers>();
    layers.0.retain(|id, _| open.contains(id));
    layers.0.insert(id, layer.clone());
    layer
}

/// Open `popover` under `anchor`. A transient one has the keyboard until it closes.
pub fn show(popover: Popover, anchor: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Handle {
    let layer = layer(window, cx);
    let handle = window.window_handle();
    let field = popover.rows.iter().find_map(|row| match row {
        Row::Field(field) => Some(field.clone()),
        _ => None,
    });
    let input = field.as_ref().map(|field| {
        let value = field.value.clone();
        let label = field.label.clone();
        cx.new(|cx| InputState::new(window, cx).default_value(value).placeholder(label))
    });
    let focus = cx.focus_handle();
    let mut events = Vec::new();
    if let (Some(input), Some(field)) = (&input, &field) {
        let weak = layer.downgrade();
        events.push(window.subscribe(input, cx, move |_, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                weak.update(cx, |layer, cx| layer.close(true, true, window, cx)).ok();
            }
        }));
        if field.focused {
            input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
    } else if popover.behavior == Behavior::Transient {
        focus.focus(window, cx);
    }
    layer.update(cx, |layer, cx| {
        layer.open =
            Some(Open { popover, anchor, field: input, says: HashMap::new(), focus, _events: events });
        cx.notify();
    });
    let weak = layer.downgrade();
    Handle::new(move |cx: &mut App| {
        let weak = weak.clone();
        // Not from inside whatever update called this.
        cx.defer(move |cx| {
            handle
                .update(cx, move |_, window, cx| {
                    weak.update(cx, |layer, cx| layer.close(false, false, window, cx)).ok();
                })
                .ok();
        });
    })
}

impl Layer {
    /// Close it: `keep` keeps the field's text (↩, a click outside), `commit_now` says it is already
    /// being committed by ↩; then `on_close` runs.
    fn close(&mut self, keep: bool, _commit_now: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open.take() else { return };
        cx.notify();
        if keep
            && let (Some(input), Some(Row::Field(field))) =
                (&open.field, open.popover.rows.iter().find(|row| matches!(row, Row::Field(_))))
        {
            let text = input.read(cx).value().to_string();
            (field.on_commit)(&text, window, cx);
        }
        if let Some(on_close) = &open.popover.on_close {
            on_close(window, cx);
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open.take() else { return };
        cx.notify();
        for row in &open.popover.rows {
            if let Row::Field(Field { on_cancel: Some(cancel), .. }) = row {
                cancel(window, cx);
            }
        }
        if let Some(on_close) = &open.popover.on_close {
            on_close(window, cx);
        }
    }
}

use crate::native::popover::Field;

impl Render for Layer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(open) = &self.open else { return Empty.into_any_element() };
        let ui = ui(cx);
        let has_path = open.popover.rows.iter().any(|row| matches!(row, Row::Path(_)));
        let transient = open.popover.behavior == Behavior::Transient;
        let mut card = style::palette_surface(ui, true)
            .id("popover-card")
            .track_focus(&open.focus)
            .key_context("Popover")
            // Over the terminal: it takes the pointer, the clicks and the wheel from it.
            .occlude()
            .w(px(if has_path { WIDTH_PATH } else { WIDTH }))
            .gap_3()
            .p_3()
            .cursor(CursorStyle::Arrow)
            .on_hover(cx.listener(|layer, hovered: &bool, window, cx| {
                let on_hover = layer.open.as_ref().and_then(|open| open.popover.on_hover.clone());
                if let Some(on_hover) = on_hover {
                    on_hover(*hovered, window, cx);
                }
            }))
            .capture_key_down(cx.listener(|layer, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && layer.open.as_ref().is_some_and(|open| open.popover.behavior == Behavior::Transient)
                {
                    cx.stop_propagation();
                    layer.cancel(window, cx);
                }
            }));
        if transient {
            card = card
                .on_mouse_down_out(cx.listener(|layer, _, window, cx| layer.close(true, false, window, cx)));
        }
        let mut button_ix = 0;
        for (row_ix, row) in open.popover.rows.iter().enumerate() {
            card = match row {
                Row::Path(text) => card.child(path_line(text, ui)),
                Row::Field(_) => card.children(open.field.as_ref().map(|input| Input::new(input).small())),
                Row::Buttons(buttons) => {
                    let mut line = div().flex().flex_wrap().gap_2();
                    for button in buttons {
                        let ix = button_ix;
                        button_ix += 1;
                        let says = open.says.get(&ix);
                        let title = says.cloned().unwrap_or_else(|| button.title.clone());
                        let glyph = if says.is_some() {
                            button.done.as_ref().and_then(|d| d.glyph)
                        } else {
                            button.glyph
                        };
                        let mut view = Button::new(("popover-button", row_ix * 8 + ix))
                            .outline()
                            .small()
                            .label(title)
                            .disabled(!button.enabled);
                        if let Some(glyph) = glyph {
                            view = view.icon(icon_of(glyph));
                        }
                        if let Some(hint) = button.hint.clone().filter(|_| !button.enabled) {
                            view = view.tooltip(hint);
                        }
                        let button = button.clone();
                        line = line.child(view.on_click(cx.listener(move |layer, _, window, cx| {
                            if button.closes {
                                layer.close(false, false, window, cx);
                                (button.on_press)(window, cx);
                            } else {
                                (button.on_press)(window, cx);
                                if let Some(done) = button.done.clone() {
                                    if let Some(open) = &mut layer.open {
                                        open.says.insert(ix, done.title.clone());
                                    }
                                    cx.notify();
                                    cx.spawn_in(window, async move |layer, cx| {
                                        cx.background_executor().timer(done.for_).await;
                                        layer
                                            .update(cx, |layer, cx| {
                                                if let Some(open) = &mut layer.open {
                                                    open.says.remove(&ix);
                                                }
                                                cx.notify();
                                            })
                                            .ok();
                                    })
                                    .detach();
                                }
                            }
                        })));
                    }
                    card.child(line)
                }
                Row::Colours(colours) => {
                    let selected = colours.swatches.selected;
                    card.child(div().flex().items_center().children(
                        colours.swatches.swatches.iter().enumerate().map(|(ix, swatch)| {
                            let on_pick = colours.swatches.on_pick.clone();
                            style::color_choice(
                                ("popover-colour", ix),
                                ui,
                                swatch.name.clone(),
                                Some(swatch.color),
                                selected == Some(ix),
                                true,
                            )
                            .on_click(move |_, window, cx| on_pick(ix, window, cx))
                        }),
                    ))
                }
                Row::Actions(items) => card.child(div().flex().flex_col().children(
                    items.iter().enumerate().map(|(ix, item)| {
                        let run = item.on_select.clone();
                        div()
                            .id(("popover-action", row_ix * 8 + ix))
                            .role(Role::Button)
                            .aria_label(item.title.clone())
                            .flex()
                            .items_center()
                            .h(px(28.0))
                            .px_2()
                            .rounded(ui.radius)
                            .text_body()
                            .when(item.enabled, |d| {
                                d.cursor_pointer().text_color(ui.text).hover(move |d| d.bg(ui.hover))
                            })
                            .when(!item.enabled, |d| d.text_color(ui.faint))
                            .child(item.title.clone())
                            .on_click(cx.listener(move |layer, _, window, cx| {
                                layer.close(false, false, window, cx);
                                if let Some(run) = &run {
                                    run(window, cx);
                                }
                            }))
                    }),
                )),
                Row::Separator => card.child(style::popover_rule(ui)),
            };
        }
        let _ = window;
        deferred(
            anchored()
                .position(open.anchor.bottom_left() + point(px(0.), px(GAP)))
                .snap_to_window_with_margin(px(8.))
                .child(card),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY)
        .into_any_element()
    }
}

fn path_line(text: &SharedString, ui: crate::settings::UiColors) -> AnyElement {
    let segments: Vec<String> = text.split_inclusive('/').map(str::to_string).collect();
    let last = segments.len().saturating_sub(1);
    div()
        .flex()
        .flex_wrap()
        .text_body()
        .children(segments.into_iter().enumerate().map(|(ix, segment)| {
            div()
                .flex_none()
                .text_color(if ix == last { ui.text } else { ui.muted })
                .when(ix == last, |d| d.font_weight(FontWeight::MEDIUM))
                .child(segment)
        }))
        .into_any_element()
}

fn icon_of(glyph: Glyph) -> gpui_kit::assets::IconName {
    use gpui_kit::assets::IconName;
    match glyph {
        Glyph::Folder => IconName::FolderOpen,
        Glyph::Copy => IconName::Copy,
        Glyph::Check => IconName::Check,
        Glyph::Terminal => IconName::SquareTerminal,
    }
}
