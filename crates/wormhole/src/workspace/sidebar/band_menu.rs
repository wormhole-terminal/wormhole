//! A folder band's context menu: New terminal here, Pin folder / Unpin folder, Colour ▸ (the
//! eight colours with a dot each, the current one checked, Custom colour… (a picker under the band) and None; all of it greyed out under
//! "Set by the project" while the project locks its colour), Reveal in Finder, Copy path. The band it was opened on wears the focus ring until it closes. ⌃↩ opens it for the
//! band the keys are on. The "No folder" band has none: there is no folder to pin or to show, and
//! its + already starts a terminal.
//!
//! A pinned folder is the person's, for every window: it is `pinned_folders` in `config.json`, so
//! pinning is a settings change and every window's sidebar redraws. A folder's colour is the
//! person's too (`folder_colors`) unless the folder's project has a `config.json`, which then holds
//! it as `project_color` (see `folder_color`).
use std::rc::Rc;

use gpui_kit::component::menu::{ContextMenu, ContextMenuExt as _, PopupMenu, PopupMenuItem};
use wormhole_core::folder_color::FolderColor;

use super::super::focus::MenuBuilder;
use super::super::folder_color::{self, FolderPaint};
use super::super::settings_page::update_settings;
use super::*;
use crate::theme_bridge;

impl Workspace {
    /// `band` with its context menu: the last thing put on a band, since the menu wraps it.
    pub(super) fn with_band_menu(
        &self,
        band: Stateful<Div>,
        folder: Option<&Path>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(folder) = folder else { return band.into_any_element() };
        let band = band.children(self.band_picker(folder));
        let build = self.band_menu(folder.to_path_buf(), cx);
        let menu: ContextMenu<Stateful<Div>> =
            band.context_menu(move |menu, window, cx| build(menu, window, cx));
        menu.into_any_element()
    }

    /// ⌃↩ with the keys on the band of `folder`: its menu, under it.
    pub(super) fn band_key_menu(&mut self, folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let listing = self.session_listing(cx);
        // The list's children are its entries, in order.
        let at = listing.entries.iter().position(|entry| {
            matches!(entry, Entry::Band { band, .. } if listing.bands[*band].folder.as_deref() == Some(&*folder))
        });
        let Some(bounds) = at.and_then(|at| self.sidebar.scroll.bounds_for_item(at)) else { return };
        let build = self.band_menu(folder, cx);
        self.open_key_menu(wormhole_core::focus::Area::Sidebar, bounds, build, window, cx);
    }

    /// What builds the menu of the band of `folder`.
    fn band_menu(&self, folder: PathBuf, cx: &mut Context<Self>) -> MenuBuilder {
        let this = cx.weak_entity();
        Rc::new(move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
            let popup = cx.entity();
            let paint = this.update(cx, |ws, cx| {
                ws.sidebar.menu_row = None;
                ws.sidebar.menu_group = None;
                ws.sidebar.menu_band = Some(folder.clone());
                let opened = folder.clone();
                ws.sidebar.menu_dismissed = Some(cx.subscribe(&popup, move |ws, _, _: &DismissEvent, cx| {
                    if ws.sidebar.menu_band.as_deref() == Some(&*opened) {
                        ws.sidebar.menu_band = None;
                        cx.notify();
                    }
                }));
                cx.notify();
                // As it is when the menu opens: another window may have changed it since the band drew.
                ws.folder_paint(&folder, cx)
            });
            let paint = paint.unwrap_or_else(|_| FolderPaint::Own(settings(cx).folder_color_choice(&folder)));
            let pinned = settings(cx).is_folder_pinned(&folder);
            let choices = folder_color::choices(cx);
            let ui = ui(cx);
            let colored = folder.clone();
            let custom = paint.custom().map(|color| {
                (theme_bridge::hsla(color.color()), format!("Custom {}", color.color().to_hex()))
            });
            let (new_here, pin, reveal, copy) =
                (folder.clone(), folder.clone(), folder.clone(), folder.clone());
            let pick_this = this.clone();
            let this = this.clone();
            menu.item(PopupMenuItem::new("New terminal here").on_click(move |_, window, cx| {
                this.update(cx, |ws, cx| ws.new_session_in(Some(new_here.clone()), window, cx)).ok();
            }))
            .separator()
            .item(
                PopupMenuItem::new(if pinned { "Unpin folder" } else { "Pin folder" })
                    .on_click(move |_, _, cx| toggle_pinned_folder(&pin, cx)),
            )
            .submenu("Colour", window, cx, move |submenu, _, _| {
                colour_choices(submenu, &colored, &paint, &choices, custom.clone(), ui, &pick_this)
            })
            .separator()
            .item(PopupMenuItem::new("Reveal in Finder").on_click(move |_, _, _| reveal_in_finder(&reveal)))
            .item(PopupMenuItem::new("Copy path").on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copy.to_string_lossy().into_owned()));
            }))
        })
    }
}

/// Pin `folder`, or unpin it: the one list of every window, saved in `config.json`.
pub(in crate::workspace) fn toggle_pinned_folder(folder: &Path, cx: &mut App) {
    update_settings(cx, |settings| {
        settings.toggle_pinned_folder(folder);
    });
}

/// The items of the Colour submenu: each colour with its dot (the current one checked), then None.
/// While the project locks its colour (or its file can't be read) the items are there but greyed
/// out, under a line saying why. A hex colour the project sets by hand is shown first, checked and
/// greyed: none of the eight is it, and the menu doesn't write hex.
fn colour_choices(
    mut menu: PopupMenu,
    folder: &Path,
    paint: &FolderPaint,
    choices: &[(FolderColor, Hsla)],
    custom: Option<(Hsla, String)>,
    ui: UiColors,
    this: &WeakEntity<Workspace>,
) -> PopupMenu {
    let closed = paint.closed();
    let current = paint.checked();
    if let Some(why) = closed {
        menu = menu.label(why);
    }
    if let Some((hsla, label)) = custom.clone() {
        menu = menu.item(
            PopupMenuItem::element(move |_, _| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(style::color_dot(ui, hsla, style::COLOR_DOT_MENU))
                    .child(label.clone())
            })
            .checked(true)
            .disabled(true),
        );
    }
    for &(color, hsla) in choices {
        let (folder, paint) = (folder.to_path_buf(), paint.clone());
        menu = menu.item(
            PopupMenuItem::element(move |_, _| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(style::color_dot(ui, hsla, style::COLOR_DOT_MENU))
                    .child(color.label())
            })
            .checked(current == Some(color))
            .disabled(closed.is_some())
            .on_click(move |_, _, cx| folder_color::set_folder_color(&paint, &folder, Some(color), cx)),
        );
    }
    let (folder, paint) = (folder.to_path_buf(), paint.clone());
    let (this, picked) = (this.clone(), folder.clone());
    menu.separator()
        .item(PopupMenuItem::new("Custom colour…").disabled(closed.is_some()).on_click(
            move |_, window, cx| {
                this.update(cx, |ws, cx| ws.start_custom_color(picked.clone(), window, cx)).ok();
            },
        ))
        .item(
            PopupMenuItem::new("None")
                .checked(current.is_none() && custom.is_none())
                .disabled(closed.is_some())
                .on_click(move |_, _, cx| folder_color::set_folder_color(&paint, &folder, None, cx)),
        )
}
