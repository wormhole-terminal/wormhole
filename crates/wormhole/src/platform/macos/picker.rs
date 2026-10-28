//! AppKit's side of `platform::picker`: a borderless, non-activating panel that is a child of the app window's
//! `NSWindow`, of Liquid Glass on macOS 26 and later, the HUD's frosted blur before it, a plain fill under
//! Reduce Transparency or Increase Contrast (the notice chip's choice of material, `notice::material`).
//!
//! Inside it, from the top: the breadcrumb (borderless buttons), a real `NSSearchField` (so the system's own
//! editing, input methods and look), the list (a few reusable row views over a view that takes the wheel) and
//! a hint line. The ghost completion is a label laid over the field where the system says the text ends.
//!
//! The panel takes the keys and the app window doesn't have them while it is up. It does it without
//! activating the app (the style is `NonactivatingPanel`), so it never comes over other apps, and in a
//! background-mode instance it only orders in, as a child of a window that is itself behind the others.
//! When it stops being the key window (a click on the terminal, another app) it tells the app (`Resigned`),
//! which closes the picker; a closing picker gives the key back to the app window.
//!
//! The theme's appearance goes on the panel's views and never on the `NSWindow`, whose appearance GPUI reads
//! as the system's (`appearance`).

use gpui_kit::{App, Window};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::sel;
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityListRole, NSAccessibilityPostNotification,
    NSAccessibilitySelectedChildrenChangedNotification, NSBezierPath, NSBox, NSBoxType, NSButton, NSColor,
    NSFocusRingType, NSFont, NSGlassEffectView, NSImage, NSImageResizingMode, NSImageScaling, NSImageView,
    NSLineBreakMode, NSSearchField, NSTextField, NSTextInputClient as _, NSTextView, NSTitlePosition, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
    NSWindowCollectionBehavior, NSWindowOrderingMode,
};
use objc2_foundation::{NSEdgeInsets, NSPoint, NSRange, NSRect, NSSize, NSString};
use wormhole_core::view::panes::Rect;

use super::appearance;
use super::appkit::{ns_window, symbol};
use super::notice::material;
use super::picker_slot::Slot;
use super::picker_views::{PickerPanel, Target};
use crate::platform::picker::{Events, Line, Material, ROW, View};

/// The panel's corners, and the border of the plain fill (the glass has an edge of its own).
const RADIUS: f64 = 18.0;
const EDGE: f64 = 1.0;
/// From the panel's top and sides to what is in it, and what each part is as tall as.
const PAD: f64 = 8.0;
const SIDE: f64 = 10.0;
const CRUMBS_H: f64 = 24.0;
const FIELD_H: f64 = 30.0;
const HINT_H: f64 = 20.0;
/// Between the parts: after the breadcrumb, after the field.
const AFTER_CRUMBS: f64 = 2.0;
const AFTER_FIELD: f64 = 6.0;
/// The list's inset from the panel's sides, so a row's selection doesn't touch the glass's edge.
const LIST_SIDE: f64 = 6.0;
/// What the breadcrumb leaves out at its start, and between steps.
const ELLIPSIS_W: f64 = 16.0;
const SEPARATOR_W: f64 = 12.0;

/// The material's view and what holds the content.
enum Backdrop {
    Glass(Retained<NSGlassEffectView>),
    Frosted(Retained<NSVisualEffectView>),
    Solid(Retained<NSBox>),
}

impl Backdrop {
    fn make(material: Material, mtm: MainThreadMarker) -> Self {
        match material {
            Material::Glass => {
                let glass = NSGlassEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                glass.setCornerRadius(RADIUS);
                Self::Glass(glass)
            }
            Material::Frosted => {
                let view = NSVisualEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                view.setMaterial(NSVisualEffectMaterial::Popover);
                view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                // Active, so the blur doesn't go flat while the app is in the background.
                view.setState(NSVisualEffectState::Active);
                view.setMaskImage(Some(&rounded()));
                Self::Frosted(view)
            }
            Material::Solid => {
                let fill = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
                fill.setBoxType(NSBoxType::Custom);
                fill.setTitlePosition(NSTitlePosition::NoTitle);
                fill.setBorderWidth(EDGE);
                fill.setCornerRadius(RADIUS);
                fill.setContentViewMargins(NSSize::new(0.0, 0.0));
                fill.setFillColor(&NSColor::windowBackgroundColor());
                fill.setBorderColor(&NSColor::separatorColor());
                Self::Solid(fill)
            }
        }
    }

    fn material(&self) -> Material {
        match self {
            Self::Glass(_) => Material::Glass,
            Self::Frosted(_) => Material::Frosted,
            Self::Solid(_) => Material::Solid,
        }
    }

    fn view(&self) -> &NSView {
        match self {
            Self::Glass(view) => view,
            Self::Frosted(view) => view,
            Self::Solid(view) => view,
        }
    }

    fn hold(&self, body: &NSView) {
        match self {
            Self::Glass(glass) => glass.setContentView(Some(body)),
            Self::Frosted(effect) => effect.addSubview(body),
            Self::Solid(fill) => {
                if let Some(content) = fill.contentView() {
                    content.addSubview(body);
                }
            }
        }
    }
}

/// An opaque rounded rectangle that stretches in the middle: the blur behind a window is cut by a mask image.
fn rounded() -> Retained<NSImage> {
    let handler = block2::RcBlock::new(move |bounds: NSRect| -> Bool {
        NSColor::blackColor().set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, RADIUS, RADIUS).fill();
        Bool::YES
    });
    let side = 2.0 * RADIUS + 2.0;
    let image = NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(side, side), false, &handler);
    image.setCapInsets(NSEdgeInsets { top: RADIUS, left: RADIUS, bottom: RADIUS, right: RADIUS });
    image.setResizingMode(NSImageResizingMode::Stretch);
    image
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
    list: Retained<super::picker_views::WheelView>,
    hint: Retained<NSTextField>,
    slots: Vec<Slot>,
    tx: Events,
    /// What was last shown, to change only what changed.
    epoch: Option<u64>,
    crumb_labels: Vec<String>,
    theme: Option<wormhole_core::theme::tokens::Appearance>,
    selected: Option<usize>,
    /// The panel has been given to the parent window.
    entered: bool,
    taken_down: bool,
    /// Where the panel is in the window's space, as last placed.
    rect: Rect,
    shown_rows: Vec<String>,
    shown_ghost: String,
}

impl Panel {
    /// A panel for `window`'s picker, not yet on screen: `opaque` for a plain fill. What happens in it goes to
    /// `events`. `None` for a window without a native one.
    pub fn new(window: &Window, opaque: bool, events: Events) -> Option<Self> {
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

        let list = super::picker_views::WheelView::new(mtm, events.clone());
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
            epoch: None,
            crumb_labels: Vec::new(),
            theme: None,
            selected: None,
            entered: false,
            taken_down: false,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            shown_rows: Vec::new(),
            shown_ghost: String::new(),
        })
    }

    /// Bring the panel up with `view`, at `rect` in the window's space, and the keyboard into its field.
    pub fn show(&mut self, view: &View<'_>, rect: Rect, cx: &mut App) {
        self.update(view, rect);
        if !self.entered {
            self.entered = true;
            // Ordering a child window makes AppKit tell the parent, whose view tells GPUI: done from a task,
            // outside this update, so GPUI is free to hear. The keys come to the panel after it is up.
            let (parent, panel, field) = (self.parent.clone(), self.panel.clone(), self.field.clone());
            cx.spawn(async move |_| adopt(&parent, &panel, &field)).detach();
        }
    }

    /// Show `view` at `rect`: only what changed is touched.
    pub fn update(&mut self, view: &View<'_>, rect: Rect) {
        self.target.set_facts(view.facts);
        if self.theme != Some(view.appearance) {
            self.theme = Some(view.appearance);
            appearance::set_on(&self.holder, view.appearance);
        }
        self.place(rect, !view.crumbs.is_empty(), view.shown);
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

    /// Take the panel down and give the key back to the app window. Closing a window tells its parent and
    /// AppKit may tell GPUI, so it is done from a task, outside whatever update this is called in.
    pub fn close(mut self, cx: &mut App) {
        self.taken_down = true;
        let (parent, panel) = (self.parent.clone(), self.panel.clone());
        cx.spawn(async move |_| take_down(&parent, &panel)).detach();
    }

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::picker::Info> {
        Some(crate::platform::picker::Info {
            material: self.backdrop.material(),
            frame: [self.rect.x, self.rect.y, self.rect.w, self.rect.h],
            key: self.panel.isKeyWindow(),
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
        self.field.setStringValue(&NSString::from_str(view.text));
        if let Some(editor) = self.field.currentEditor() {
            editor.setSelectedRange(NSRange::new(view.text.encode_utf16().count(), 0));
        }
    }

    fn set_crumbs(&mut self, view: &View<'_>) {
        let labels: Vec<String> = view.crumbs.iter().map(|c| c.label.clone()).collect();
        if labels == self.crumb_labels {
            return;
        }
        self.crumb_labels = labels;
        let Some(mtm) = MainThreadMarker::new() else { return };
        for old in self.crumbs.subviews().iter() {
            old.removeFromSuperview();
        }
        let buttons: Vec<Retained<NSButton>> = view
            .crumbs
            .iter()
            .enumerate()
            .map(|(at, crumb)| crumb_button(mtm, &crumb.label, at, at + 1 == view.crumbs.len(), &self.target))
            .collect();
        let widths: Vec<f64> = buttons.iter().map(|b| b.frame().size.width).collect();
        let room = self.crumbs.frame().size.width;
        let first = first_crumb(&widths, room);
        let mut x = 0.0;
        if first > 0 {
            let dots = NSTextField::labelWithString(&NSString::from_str("…"), mtm);
            dots.setFont(Some(&NSFont::systemFontOfSize(12.0)));
            dots.setTextColor(Some(&NSColor::tertiaryLabelColor()));
            dots.setFrame(NSRect::new(NSPoint::new(x, 3.0), NSSize::new(ELLIPSIS_W, 17.0)));
            self.crumbs.addSubview(&dots);
            x += ELLIPSIS_W;
        }
        for (at, button) in buttons.iter().enumerate().skip(first) {
            if at > first {
                let chevron = NSImageView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                chevron.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                chevron.setImage(symbol("chevron.right", "").as_deref());
                chevron.setContentTintColor(Some(&NSColor::tertiaryLabelColor()));
                chevron.setFrame(NSRect::new(NSPoint::new(x + 2.0, 7.0), NSSize::new(8.0, 10.0)));
                chevron.setAccessibilityElement(false);
                self.crumbs.addSubview(&chevron);
                x += SEPARATOR_W;
            }
            let size = button.frame().size;
            button.setFrame(NSRect::new(NSPoint::new(x, ((CRUMBS_H - size.height) / 2.0).floor()), size));
            self.crumbs.addSubview(button);
            x += size.width;
        }
    }

    fn set_rows(&mut self, view: &View<'_>) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let width = self.list.frame().size.width;
        let end = (view.top + view.shown).min(view.lines.len());
        let in_view = view.lines.get(view.top.min(end)..end).unwrap_or_default();
        while self.slots.len() < in_view.len() {
            let slot = Slot::new(mtm, self.tx.clone());
            self.list.addSubview(&slot.view);
            self.slots.push(slot);
        }
        self.shown_rows.clear();
        for (i, slot) in self.slots.iter().enumerate() {
            let Some(line) = in_view.get(i) else {
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

/// A borderless button for a step of the breadcrumb: the current one in the text colour, the others quieter.
fn crumb_button(
    mtm: MainThreadMarker,
    label: &str,
    at: usize,
    current: bool,
    target: &Target,
) -> Retained<NSButton> {
    // SAFETY: `crumb:` is defined on `Target` with the `(id)sender` signature of an action; the panel keeps
    // the target alive, and AppKit holds it weakly.
    #[allow(unsafe_code)]
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(label),
            Some(target),
            Some(sel!(crumb:)),
            mtm,
        )
    };
    button.setBordered(false);
    button.setFont(Some(&NSFont::systemFontOfSize(12.0)));
    button.setTag(at as isize);
    let tint = if current { NSColor::labelColor() } else { NSColor::secondaryLabelColor() };
    button.setContentTintColor(Some(&tint));
    button.sizeToFit();
    button
}

/// Which step the breadcrumb starts at so the steps from it fit in `room`: all of them when they do, else the
/// last ones with room for the ellipsis that stands for the rest (and always at least the last).
fn first_crumb(widths: &[f64], room: f64) -> usize {
    for first in 0..widths.len() {
        let steps = widths[first..].iter().sum::<f64>() + SEPARATOR_W * (widths.len() - first - 1) as f64;
        let dots = if first > 0 { ELLIPSIS_W } else { 0.0 };
        if steps + dots <= room {
            return first;
        }
    }
    widths.len().saturating_sub(1)
}

impl Drop for Panel {
    fn drop(&mut self) {
        if !self.taken_down {
            take_down(&self.parent, &self.panel);
        }
    }
}

/// Give `panel` to `parent` as a child, above it, and the keyboard to `field`.
#[allow(unsafe_code)]
fn adopt(parent: &NSWindow, panel: &PickerPanel, field: &NSSearchField) {
    // SAFETY: both windows are live and on the main thread (the executor's).
    unsafe { parent.addChildWindow_ordered(panel, NSWindowOrderingMode::Above) };
    panel.makeKeyAndOrderFront(None);
    panel.makeFirstResponder(Some(field));
}

/// Take `panel` off `parent` and close it; the app window is the key window again if the panel was.
fn take_down(parent: &NSWindow, panel: &PickerPanel) {
    let had_the_key = panel.isKeyWindow();
    parent.removeChildWindow(panel);
    panel.orderOut(None);
    panel.close();
    if had_the_key {
        parent.makeKeyWindow();
    }
}

#[cfg(test)]
mod tests {
    use objc2_foundation::NSSize;

    use super::{CRUMBS_H, ELLIPSIS_W, FIELD_H, PAD, SEPARATOR_W, first_crumb, layout};
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

    #[test]
    fn a_breadcrumb_that_fits_starts_at_its_first_step_and_a_long_one_drops_its_start() {
        let widths = [20.0, 60.0, 80.0, 70.0];
        assert_eq!(first_crumb(&widths, 400.0), 0);
        let all = 20.0 + 60.0 + 80.0 + 70.0 + 3.0 * SEPARATOR_W;
        assert_eq!(
            first_crumb(&widths, all - 1.0),
            1,
            "the ellipsis takes {ELLIPSIS_W} of what the first gave back"
        );
        assert_eq!(first_crumb(&widths, 10.0), 3, "at least the last step");
        assert_eq!(first_crumb(&[], 10.0), 0);
    }
}
