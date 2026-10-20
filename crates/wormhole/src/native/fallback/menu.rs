//! A [`Menu`] as GPUI Kit's `PopupMenu`, drawn over the window by a layer the workspace renders:
//! for platforms without a native menu, and for a window with no native view to pop one up in.
//! Keys are not shown here; the Kit's popup shows an action's key, and a menu item carries none.

use std::collections::HashMap;

use gpui_kit::assets::IconName;
use gpui_kit::base::POPUP_PRIORITY;
use gpui_kit::base::actions::Cancel;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::*;

use crate::native::menu::{Check, Entry, Icon, Menu, Swatches};

/// The open menu of one window, and the element that draws it.
#[derive(Default)]
pub struct Layer {
    open: Option<Open>,
}

struct Open {
    popup: Entity<PopupMenu>,
    at: Point<Pixels>,
    _dismissed: Subscription,
}

/// The layers of the windows that have had one.
#[derive(Default)]
struct Layers(HashMap<WindowId, Entity<Layer>>);

impl Global for Layers {}

/// `window`'s layer: the workspace puts it in its tree, [`show`] opens menus in it.
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

/// Open `menu` at `at`; the menu has the keyboard until it closes, then the focus goes back to
/// where it was unless the chosen item put it somewhere else.
pub fn show(menu: Menu, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let layer = layer(window, cx);
    let previous = window.focused(cx);
    let on_close = menu.on_close.clone();
    let popup = PopupMenu::build(window, cx, move |popup, window, cx| fill(popup, &menu, window, cx));
    popup.focus_handle(cx).focus(window, cx);
    let popup_focus = popup.focus_handle(cx);
    let weak = layer.downgrade();
    let dismissed = window.subscribe(&popup, cx, move |_, _: &DismissEvent, window, cx| {
        weak.update(cx, |layer, cx| {
            layer.open = None;
            cx.notify();
        })
        .ok();
        if let Some(on_close) = &on_close {
            on_close(window, cx);
        }
        let kept = window.focused(cx).is_some_and(|focused| !popup_focus.contains(&focused, window));
        if !kept && let Some(previous) = &previous {
            previous.focus(window, cx);
        }
    });
    layer.update(cx, |layer, cx| {
        layer.open = Some(Open { popup, at, _dismissed: dismissed });
        cx.notify();
    });
}

impl Render for Layer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let Some(open) = &self.open else { return Empty.into_any_element() };
        deferred(anchored().position(open.at).snap_to_window_with_margin(px(8.)).child(open.popup.clone()))
            .with_priority(POPUP_PRIORITY)
            .into_any_element()
    }
}

fn fill(mut popup: PopupMenu, menu: &Menu, window: &mut Window, cx: &mut Context<PopupMenu>) -> PopupMenu {
    for entry in &menu.entries {
        popup = match entry {
            Entry::Separator => popup.separator(),
            Entry::Header(title) => popup.label(title.clone()),
            Entry::Item(item) => {
                let run = item.on_select.clone();
                let click = move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    if let Some(run) = &run {
                        run(window, cx);
                    }
                };
                let shown = match &item.icon {
                    None => PopupMenuItem::new(item.title.clone()),
                    Some(icon) => {
                        let (title, icon) = (item.title.clone(), icon.clone());
                        PopupMenuItem::element(move |_, _| with_icon(&icon, title.clone()))
                    }
                };
                popup.item(shown.checked(item.check != Check::Off).disabled(!item.enabled).on_click(click))
            }
            Entry::Swatches(row) => {
                let row = row.clone();
                popup.item(PopupMenuItem::element(move |_, cx| dots(&row, cx)))
            }
            Entry::Submenu { title, enabled, menu } => {
                if *enabled {
                    let menu = menu.clone();
                    popup.submenu(title.clone(), window, cx, move |sub, window, cx| {
                        fill(sub, &menu, window, cx)
                    })
                } else {
                    popup.item(PopupMenuItem::new(title.clone()).disabled(true))
                }
            }
        };
    }
    popup
}

fn with_icon(icon: &Icon, title: SharedString) -> AnyElement {
    let picture = match icon {
        Icon::Image(file) => img(file.clone()).size_4().into_any_element(),
        // No icon service here: the folder glyph stands for any file.
        Icon::File(_) => crate::style::icon(IconName::Folder).into_any_element(),
    };
    div().flex().items_center().gap_2().child(picture).child(title).into_any_element()
}

/// A row of colour dots: a click on one picks it and dismisses the menu, as the native row does.
fn dots(row: &Swatches, cx: &App) -> AnyElement {
    let ui = crate::settings::ui(cx);
    div()
        .flex()
        .items_center()
        .children(row.swatches.iter().enumerate().map(|(ix, swatch)| {
            let on_pick = row.on_pick.clone();
            crate::style::swatch(("menu-swatch", ix), ui, swatch.color, row.selected == Some(ix))
                .role(Role::Button)
                .aria_label(swatch.name.clone())
                .on_click(move |_, window, cx| {
                    on_pick(ix, window, cx);
                    window.dispatch_action(Box::new(Cancel), cx);
                })
        }))
        .into_any_element()
}
