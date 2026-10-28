//! AppKit's side of `platform::picker`: a borderless, non-activating panel that is a child of the app window's
//! `NSWindow`, of Liquid Glass on macOS 26 and later, the HUD's frosted blur before it, a plain fill under
//! Reduce Transparency or Increase Contrast (`picker_backdrop`).
//!
//! Inside it, from the top: the breadcrumb (`picker_crumbs`), a real `NSSearchField` (so the system's own
//! editing, input methods and look), the list (a few reusable row views, `picker_slot`, over a view that takes
//! the wheel) and a hint line. The ghost completion is a label laid over the field where the system says the
//! text ends.
//!
//! The panel takes the keys and the app window doesn't have them while it is up, without activating the app
//! (the style is `NonactivatingPanel`). That is only done when the app is the active one and this is not a
//! background-mode instance ([`arrival`]): taking the keys from whoever is in front is not what a test instance
//! or an app that was switched away from meanwhile may do. In background mode the panel is not shown at all, so
//! it can come over nothing and takes no keystroke; in an app that is no longer active it is not shown either
//! and the picker is told it is over. When the panel stops being the key window (a click on the terminal,
//! another app) it tells the app (`Resigned`), which closes the picker.
//!
//! A panel is taken down once, from a task and never from inside GPUI's release of the entity that holds it:
//! ordering windows tells GPUI, and handing the key back would be a key change in the middle of a release. What
//! the deferred ordering-in does is checked against that: a picker that was closed before it ran leaves the
//! panel closed.
//!
//! The theme's appearance goes on the panel's views and never on the `NSWindow`, whose appearance GPUI reads
//! as the system's (`appearance`).

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{App, ForegroundExecutor, Window};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::sel;
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityListRole, NSAccessibilityPostNotification,
    NSAccessibilitySelectedChildrenChangedNotification, NSApplication, NSColor, NSFocusRingType, NSFont,
    NSLineBreakMode, NSSearchField, NSTextField, NSTextInputClient as _, NSTextView, NSView, NSWindow,
    NSWindowCollectionBehavior, NSWindowOrderingMode,
};
use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};
use wormhole_core::view::panes::Rect;

use super::appearance;
use super::appkit::ns_window;
use super::notice::material;
use super::picker_backdrop::Backdrop;
use super::picker_crumbs::{self, HEIGHT as CRUMBS_H};
use super::picker_slot::Slot;
use super::picker_views::{PickerPanel, Target, WheelView};
use crate::platform::background_mode;
use crate::platform::picker::{Event, Events, Line, ROW, View};

/// From the panel's top and sides to what is in it, and what each part is as tall as.
const PAD: f64 = 8.0;
const SIDE: f64 = 10.0;
const FIELD_H: f64 = 30.0;
const HINT_H: f64 = 20.0;
/// Between the parts: after the breadcrumb, after the field.
const AFTER_CRUMBS: f64 = 2.0;
const AFTER_FIELD: f64 = 6.0;
/// The list's inset from the panel's sides, so a row's selection doesn't touch the glass's edge.
const LIST_SIDE: f64 = 6.0;

/// What becomes of the panel when it is time to put it on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arrival {
    /// It is not shown: a background-mode instance never comes over anything and takes no keys.
    Hidden,
    /// It is not shown and the picker is over: the app was switched away from since the picker opened.
    TooLate,
    /// It is ordered in over the app's window and takes the keys.
    Key,
}

/// What to do with the panel for an instance in background mode (or not) whose app is active (or not).
fn arrival(background: bool, app_active: bool) -> Arrival {
    if background {
        Arrival::Hidden
    } else if app_active {
        Arrival::Key
    } else {
        Arrival::TooLate
    }
}

/// Where each part goes in a panel `size` big, in the panel's space (AppKit's: from the bottom left).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
    crumbs: Option<NSRect>,
    field: NSRect,
    list: NSRect,
    hint: NSRect,
}

fn layout(size: NSSize, has_crumbs: bool, rows: usize) -> Layout {
    let inner = (size.width - 2.0 * SIDE).max(0.0);
    let from_top = |top: f64, height: f64| size.height - top - height;
    let mut top = PAD;
    let crumbs = has_crumbs.then(|| {
        let at = NSRect::new(NSPoint::new(SIDE, from_top(top, CRUMBS_H)), NSSize::new(inner, CRUMBS_H));
        top += CRUMBS_H + AFTER_CRUMBS;
        at
    });
    let field = NSRect::new(NSPoint::new(SIDE, from_top(top, FIELD_H)), NSSize::new(inner, FIELD_H));
    top += FIELD_H + AFTER_FIELD;
    let height = rows as f64 * f64::from(ROW);
    let list = NSRect::new(
        NSPoint::new(LIST_SIDE, from_top(top, height)),
        NSSize::new((size.width - 2.0 * LIST_SIDE).max(0.0), height),
    );
    let hint = NSRect::new(NSPoint::new(SIDE + 4.0, PAD), NSSize::new((inner - 8.0).max(0.0), HINT_H));
    Layout { crumbs, field, list, hint }
}

/// One window's picker panel.
pub struct Panel {
    parent: Retained<NSWindow>,
    panel: Retained<PickerPanel>,
    target: Retained<Target>,
    holder: Retained<NSView>,
    backdrop: Backdrop,
    body: Retained<NSView>,
    crumbs: Retained<NSView>,
    field: Retained<NSSearchField>,
    ghost: Retained<NSTextField>,
    list: Retained<WheelView>,
    hint: Retained<NSTextField>,
    slots: Vec<Slot>,
    tx: Events,
    /// Runs what must wait for GPUI to be free of the panel's owner.
    executor: ForegroundExecutor,
    /// What was last shown, to change only what changed.
    epoch: Option<u64>,
    crumb_labels: Vec<String>,
    crumb_room: f64,
    theme: Option<wormhole_core::theme::tokens::Appearance>,
    selected: Option<usize>,
    /// The panel has been given to the parent window (asked to be).
    entered: bool,
    /// The panel has been taken down, or will be before anything else runs: a deferred ordering-in is
    /// cancelled by it. Shared with that task.
    gone: Rc<Cell<bool>>,
    /// Where the panel is in the window's space, as last placed.
    rect: Rect,
    shown_rows: Vec<String>,
    shown_ghost: String,
}

impl Panel {
    /// A panel for `window`'s picker, not yet on screen: `opaque` for a plain fill. What happens in it goes to
    /// `events`. `None` for a window without a native one.
    pub fn new(window: &Window, opaque: bool, events: Events, cx: &App) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let parent = ns_window(window)?;
        let panel = PickerPanel::new(mtm);
        // SAFETY: on the main thread (`mtm`), and the panel is kept by `Retained`, so AppKit must not also
        // release it when it closes.
        #[allow(unsafe_code)]
        unsafe {
            panel.setReleasedWhenClosed(false);
        }
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(true);
        panel.setHidesOnDeactivate(false);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::Transient
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );

        let target = Target::new(mtm, events.clone());
        panel.setDelegate(Some(ProtocolObject::from_ref(&*target)));

        let holder = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        holder.setWantsLayer(true);
        panel.setContentView(Some(&holder));
        let backdrop = Backdrop::make(material(opaque), mtm);
        holder.addSubview(backdrop.view());
        let body = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        backdrop.hold(&body);

        let crumbs = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        let field = NSSearchField::initWithFrame(mtm.alloc(), NSRect::ZERO);
        field.setFont(Some(&NSFont::systemFontOfSize(15.0)));
        field.setFocusRingType(NSFocusRingType::None);
        field.setSendsSearchStringImmediately(true);
        // SAFETY: the delegate and the target are weak in AppKit; the panel keeps the target alive. `searched:`
        // is defined on `Target` with the `(id)sender` signature of an action.
        #[allow(unsafe_code)]
        unsafe {
            NSTextField::setDelegate(&field, Some(ProtocolObject::from_ref(&*target)));
            field.setTarget(Some(&*target));
            field.setAction(Some(sel!(searched:)));
        }
        target.watch(&field);

        let ghost = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        ghost.setFont(Some(&NSFont::systemFontOfSize(15.0)));
        ghost.setTextColor(Some(&NSColor::placeholderTextColor()));
        ghost.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        ghost.setHidden(true);
        // The ghost is decoration: VoiceOver reads the field and the list, not a second copy of the text.
        ghost.setAccessibilityElement(false);

        let list = WheelView::new(mtm, events.clone());
        list.setAccessibilityElement(true);
        // SAFETY: a constant AppKit defines, read on the main thread.
        #[allow(unsafe_code)]
        list.setAccessibilityRole(Some(unsafe { NSAccessibilityListRole }));
        let hint = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        hint.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        hint.setTextColor(Some(&NSColor::tertiaryLabelColor()));
        hint.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        hint.setAccessibilityElement(false);

        body.addSubview(&crumbs);
        body.addSubview(&field);
        body.addSubview(&list);
        body.addSubview(&hint);
        // Over the field, last, so it is drawn after the field's own text.
        body.addSubview(&ghost);

        Some(Self {
            parent,
            panel,
            target,
            holder,
            backdrop,
            body,
            crumbs,
            field,
            ghost,
            list,
            hint,
            slots: Vec::new(),
            tx: events,
            executor: cx.foreground_executor().clone(),
            epoch: None,
            crumb_labels: Vec::new(),
            crumb_room: 0.0,
            theme: None,
            selected: None,
            entered: false,
            gone: Rc::new(Cell::new(false)),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            shown_rows: Vec::new(),
            shown_ghost: String::new(),
        })
    }

    /// Bring the panel up with `view`, at `rect` in the window's space, and the keyboard into its field (see
    /// [`arrival`] for when it does and does not).
    pub fn show(&mut self, view: &View<'_>, rect: Rect, cx: &mut App) {
        self.update(view, rect);
        if !self.entered {
            self.entered = true;
            // Ordering a child window makes AppKit tell the parent, whose view tells GPUI: done from a task,
            // outside this update, so GPUI is free to hear. The keys come to the panel after it is up.
            let (parent, panel, field) = (self.parent.clone(), self.panel.clone(), self.field.clone());
            let (gone, tx) = (self.gone.clone(), self.tx.clone());
            cx.spawn(async move |_| adopt(&parent, &panel, &field, &gone, &tx)).detach();
        }
    }

    /// Show `view` at `rect`: only what changed is touched.
    pub fn update(&mut self, view: &View<'_>, rect: Rect) {
        self.target.set_facts(view.facts);
        if self.theme != Some(view.appearance) {
            self.theme = Some(view.appearance);
            appearance::set_on(&self.holder, view.appearance);
        }
        self.place(rect, !view.crumbs.is_empty(), view.lines.len());
        self.set_text(view);
        self.set_crumbs(view);
        self.set_rows(view);
        self.hint.setStringValue(&NSString::from_str(view.hint));
        self.field.setPlaceholderString(Some(&NSString::from_str(view.placeholder)));
        self.field.setAccessibilityLabel(Some(&NSString::from_str(view.placeholder)));
        self.panel.setTitle(&NSString::from_str(view.name));
        self.list.setAccessibilityLabel(Some(&NSString::from_str(view.name)));
        self.set_ghost(view);
    }

    /// Take the panel down and give the key back to the app window, from a task.
    pub fn close(self, _cx: &mut App) {
        // Dropping does it: one way to take the panel down, whoever lets go of it.
        drop(self);
    }

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::picker::Info> {
        Some(crate::platform::picker::Info {
            material: self.backdrop.material(),
            frame: [self.rect.x, self.rect.y, self.rect.w, self.rect.h],
            key: self.panel.isKeyWindow(),
            visible: self.panel.isVisible(),
            window_number: self.panel.windowNumber() as i64,
            parent_number: self.parent.windowNumber() as i64,
            ghost: self.shown_ghost.clone(),
            rows: self.shown_rows.clone(),
            crumbs: self.crumb_labels.clone(),
        })
    }

    /// The panel at `rect` (the window's space, from its top left) over the window, and the parts in it.
    fn place(&mut self, rect: Rect, has_crumbs: bool, rows: usize) {
        self.rect = rect;
        let parent = self.parent.frame();
        let size = NSSize::new(f64::from(rect.w), f64::from(rect.h));
        let frame = NSRect::new(
            NSPoint::new(
                parent.origin.x + f64::from(rect.x),
                parent.origin.y + parent.size.height - f64::from(rect.y) - size.height,
            ),
            size,
        );
        if self.panel.frame() != frame {
            self.panel.setFrame_display(frame, true);
        }
        let whole = NSRect::new(NSPoint::ZERO, size);
        for view in [&self.holder, &self.body] {
            if view.frame() != whole {
                view.setFrame(whole);
            }
        }
        self.backdrop.view().setFrame(whole);
        let parts = layout(size, has_crumbs, rows);
        self.crumbs.setHidden(parts.crumbs.is_none());
        if let Some(at) = parts.crumbs {
            self.crumbs.setFrame(at);
        }
        self.field.setFrame(parts.field);
        self.list.setFrame(parts.list);
        self.hint.setFrame(parts.hint);
    }

    /// The field's text, when the picker (not the person) changed it.
    fn set_text(&mut self, view: &View<'_>) {
        if self.epoch == Some(view.epoch) {
            return;
        }
        self.epoch = Some(view.epoch);
        // Edits the field reports from now on are of this text, which is how an edit that was on its way from
        // the text it replaced is told apart (`Event::Typed`).
        self.target.set_epoch(view.epoch);
        self.field.setStringValue(&NSString::from_str(view.text));
        if let Some(editor) = self.field.currentEditor() {
            editor.setSelectedRange(NSRange::new(view.text.encode_utf16().count(), 0));
        }
    }

    /// The breadcrumb, laid out again when its steps or the room for them changed.
    fn set_crumbs(&mut self, view: &View<'_>) {
        let labels: Vec<String> = view.crumbs.iter().map(|c| c.label.clone()).collect();
        let room = self.crumbs.frame().size.width;
        if labels == self.crumb_labels && (room - self.crumb_room).abs() < 0.5 {
            return;
        }
        self.crumb_labels = labels;
        self.crumb_room = room;
        picker_crumbs::fill(&self.crumbs, view.crumbs, &self.target, room);
    }

    /// The lines in view, which are all `view` has, from line number `view.top`.
    fn set_rows(&mut self, view: &View<'_>) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let width = self.list.frame().size.width;
        while self.slots.len() < view.lines.len() {
            let slot = Slot::new(mtm, self.tx.clone());
            self.list.addSubview(&slot.view);
            self.slots.push(slot);
        }
        self.shown_rows.clear();
        for (i, slot) in self.slots.iter().enumerate() {
            let Some(line) = view.lines.get(i) else {
                slot.view.setHidden(true);
                continue;
            };
            let at = view.top + i;
            slot.view.setHidden(false);
            slot.fill(at, *line, view.selected == Some(at), width);
            let mut frame = slot.view.frame();
            frame.origin.y = i as f64 * f64::from(ROW);
            slot.view.setFrame(frame);
            if let Line::Row(row) = line {
                self.shown_rows.push(row.label.clone());
            }
        }
        // VoiceOver is told when the selection moved, so it reads the new row.
        if self.selected != view.selected {
            self.selected = view.selected;
            // SAFETY: the element is a live view and the name is AppKit's constant.
            #[allow(unsafe_code)]
            unsafe {
                NSAccessibilityPostNotification(
                    &self.list,
                    NSAccessibilitySelectedChildrenChangedNotification,
                );
            }
        }
    }

    /// The ghost completion where the field's text ends, if the field is being edited and there is one.
    fn set_ghost(&mut self, view: &View<'_>) {
        self.shown_ghost.clear();
        let Some(ghost) = view.ghost.filter(|g| !g.is_empty()) else {
            self.ghost.setHidden(true);
            return;
        };
        let Some(at) = self.text_end() else {
            self.ghost.setHidden(true);
            return;
        };
        self.ghost.setStringValue(&NSString::from_str(ghost));
        self.ghost.sizeToFit();
        let field = self.field.frame();
        // The search field keeps room at its right for the clear button.
        let right = field.origin.x + field.size.width - 26.0;
        let width = (right - at.x).max(0.0).min(self.ghost.frame().size.width);
        let height = self.ghost.frame().size.height;
        self.ghost.setFrame(NSRect::new(
            NSPoint::new(at.x, field.origin.y + ((field.size.height - height) / 2.0).floor()),
            NSSize::new(width, height),
        ));
        self.ghost.setHidden(width < 4.0);
        self.shown_ghost = ghost.to_string();
    }

    /// Where the field's text ends, in the body's space: asked of the field's text view, which knows how the
    /// text is laid out (scrolled, with the search icon before it).
    fn text_end(&self) -> Option<NSPoint> {
        let editor = self.field.currentEditor()?;
        let text_view = editor.downcast::<NSTextView>().ok()?;
        let end = text_view.string().to_string().encode_utf16().count();
        // SAFETY: a null `actualRange` is allowed.
        #[allow(unsafe_code)]
        let on_screen = unsafe {
            text_view.firstRectForCharacterRange_actualRange(NSRange::new(end, 0), std::ptr::null_mut())
        };
        let in_window = self.panel.convertRectFromScreen(on_screen);
        Some(self.body.convertRect_fromView(in_window, None).origin)
    }
}

impl Drop for Panel {
    /// The one way the panel is taken down: from a task, after whatever update this is part of, so GPUI is not
    /// told of a window going or of the key moving while it releases the entity that held the panel.
    fn drop(&mut self) {
        if self.gone.replace(true) {
            return;
        }
        let (parent, panel) = (self.parent.clone(), self.panel.clone());
        self.executor.spawn(async move { take_down(&parent, &panel) }).detach();
    }
}

/// Give `panel` to `parent` as a child, above it, and the keyboard to `field`, if the panel is still wanted and
/// [`arrival`] says so.
#[allow(unsafe_code)]
fn adopt(parent: &NSWindow, panel: &PickerPanel, field: &NSSearchField, gone: &Cell<bool>, tx: &Events) {
    // A picker closed before this ran has no panel to show: ordering it in would leave a window nobody owns.
    if gone.get() {
        return;
    }
    let active = MainThreadMarker::new().is_some_and(|mtm| NSApplication::sharedApplication(mtm).isActive());
    match arrival(background_mode::on(), active) {
        Arrival::Hidden => {}
        Arrival::TooLate => {
            tx.try_send(Event::Resigned).ok();
        }
        Arrival::Key => {
            // SAFETY: both windows are live and on the main thread (the executor's).
            unsafe { parent.addChildWindow_ordered(panel, NSWindowOrderingMode::Above) };
            panel.makeKeyAndOrderFront(None);
            panel.makeFirstResponder(Some(field));
        }
    }
}

/// Take `panel` off `parent` and close it; the app window is the key window again if the panel was, and it is
/// on screen to take it.
fn take_down(parent: &NSWindow, panel: &PickerPanel) {
    let had_the_key = panel.isKeyWindow();
    parent.removeChildWindow(panel);
    panel.orderOut(None);
    panel.close();
    if had_the_key && parent.isVisible() {
        parent.makeKeyWindow();
    }
}

#[cfg(test)]
mod tests {
    use objc2_foundation::NSSize;

    use super::{Arrival, CRUMBS_H, FIELD_H, PAD, arrival, layout};
    use crate::platform::picker::{ROW, card};

    #[test]
    fn the_parts_stack_from_the_top_and_fill_the_cards_height() {
        for has_crumbs in [false, true] {
            let rows = 5;
            let c = card(has_crumbs);
            let size = NSSize::new(f64::from(c.width), f64::from(c.height(rows)));
            let l = layout(size, has_crumbs, rows);
            // Nothing leaves the panel and nothing overlaps: field above list above hint.
            assert!(l.field.origin.y >= l.list.origin.y + l.list.size.height);
            assert!(l.list.origin.y >= l.hint.origin.y + l.hint.size.height);
            assert_eq!(l.list.size.height, rows as f64 * f64::from(ROW));
            assert_eq!(
                l.field.origin.y + FIELD_H + if has_crumbs { CRUMBS_H + 2.0 } else { 0.0 } + PAD,
                size.height
            );
            assert_eq!(l.crumbs.is_some(), has_crumbs);
        }
    }

    #[test]
    fn the_card_is_as_tall_as_the_layout_needs() {
        // Six points between the list and the hint line: what the card's chrome adds to the hint's height.
        let c = card(false);
        let size = NSSize::new(f64::from(c.width), f64::from(c.height(3)));
        let l = layout(size, false, 3);
        assert_eq!(l.list.origin.y - (l.hint.origin.y + l.hint.size.height), 4.0);
    }

    /// The panel takes the keys only in an app that is active and not in background mode: a test instance
    /// never takes a keystroke from the person's own app, and an app switched away from meanwhile does not
    /// either.
    #[test]
    fn background_mode_never_gets_the_keys_whatever_the_app_is_doing() {
        for active in [true, false] {
            assert_eq!(arrival(true, active), Arrival::Hidden, "background mode, active {active}");
        }
        assert_eq!(arrival(false, true), Arrival::Key);
        assert_eq!(arrival(false, false), Arrival::TooLate, "switched away from since it opened: not shown");
    }
}
