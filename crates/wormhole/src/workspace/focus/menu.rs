//! ⌃↩ in a list opens the row's context menu from the keyboard: the menu of the row the keys are
//! on, the one a right-click shows, opened under that row as macOS's lists do under Full Keyboard
//! Access. The keyboard goes to the menu and comes back to the list when it closes. A list whose
//! rows have no menu yet (the panel's, the tabs') still takes the key, so it never reaches the
//! terminal behind it.

use std::rc::Rc;

use gpui_kit::base::POPUP_PRIORITY;
use gpui_kit::component::menu::PopupMenu;

use super::*;

/// How far inside the window the menu stays, as the Kit keeps a right-click's.
const WINDOW_MARGIN: f32 = 8.0;

/// What builds a row's menu: the right-click's own builder.
pub(in crate::workspace) type MenuBuilder =
    Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

/// A row's menu opened from the keyboard.
pub(in crate::workspace) struct KeyMenu {
    /// The area whose list it belongs to: it is drawn from that area's container.
    area: Area,
    /// Its top left corner: under the row, at the row's start.
    at: Point<Pixels>,
    menu: Entity<PopupMenu>,
    _dismissed: Subscription,
}

/// Whether `keystroke` is the row menu's key.
pub(super) fn is_menu_key(keystroke: &Keystroke) -> bool {
    keymap::list_action(&crate::terminal_view::keystroke_spec(keystroke)) == Some(ActionId::RowMenu)
}

impl Workspace {
    /// ⌃↩ in `area`'s list: open the menu of the row the keys are on, if it has one.
    pub(super) fn row_menu_key(&mut self, area: Area, window: &mut Window, cx: &mut Context<Self>) {
        // The panel's rows and the terminal's tabs have no menu yet.
        if area == Area::Sidebar {
            self.sidebar_key_menu(window, cx);
        }
    }

    /// Open `build`'s menu for the row at `row` in `area`'s list, the keyboard on it. The menu is
    /// built after this update, as a right-click's is: its builder may update the workspace.
    pub(in crate::workspace) fn open_key_menu(
        &mut self,
        area: Area,
        row: Bounds<Pixels>,
        build: MenuBuilder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(list) = self.areas.handle(area).cloned() else { return };
        let this = cx.weak_entity();
        // Under the row, in from its edge by the list's inset (a flush row starts at the window's
        // edge, where the menu's corners would be cut off): where a right-click on its lead
        // column would put it.
        let at = point(row.left() + px(style::SIDEBAR_INSET), row.bottom());
        window.defer(cx, move |window, cx| {
            // Dismissed, it gives the keyboard back to the list.
            let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                build(menu.action_context(list), window, cx)
            });
            if menu.read(cx).is_empty() {
                return;
            }
            menu.focus_handle(cx).focus(window, cx);
            this.update(cx, |ws, cx| {
                let dismissed = cx.subscribe(&menu, |ws, _, _: &DismissEvent, cx| {
                    ws.areas.menu = None;
                    cx.notify();
                });
                ws.areas.menu = Some(KeyMenu { area, at, menu, _dismissed: dismissed });
                cx.notify();
            })
            .ok();
        });
    }

    /// The menu ⌃↩ opened in `area`'s list, drawn over everything.
    pub(super) fn key_menu_layer(&self, area: Area) -> Option<AnyElement> {
        let open = self.areas.menu.as_ref().filter(|menu| menu.area == area)?;
        let menu = anchored()
            .position(open.at)
            .snap_to_window_with_margin(px(WINDOW_MARGIN))
            .child(open.menu.clone());
        Some(deferred(menu).with_priority(POPUP_PRIORITY).into_any_element())
    }
}
