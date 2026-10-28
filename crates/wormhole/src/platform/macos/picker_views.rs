//! The picker panel's own classes, and the small conversions they share: the panel itself (which may be the key
//! window without being main), the object that is every control's delegate and target, a row that can be
//! clicked and a view that takes the wheel.
//!
//! None of them runs app code. What the person did is put in the queue the app reads (`platform::picker::Events`),
//! except one answer AppKit wants before the key returns: whether the picker takes an arrow key or the field
//! keeps it. That is core's `decide`, asked here with the facts the panel was last given.

use std::cell::{Cell, RefCell};
use std::ops::Range;

use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, Sel};
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility as _, NSBackingStoreType, NSButton, NSControl, NSControlTextEditingDelegate, NSEvent,
    NSPanel, NSSearchField, NSTextFieldDelegate, NSTextView, NSView, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{NSNotification, NSObject, NSRange, NSRect};
use wormhole_core::view::picker::{Do, Facts, Field, Key, decide};

use crate::platform::picker::{Event, Events, ROW};

/// `text[range]` for a range of UTF-16 units, as AppKit counts, as byte offsets. An offset inside a surrogate
/// pair is taken to the end of the character.
pub fn bytes(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (at, c) in text.char_indices() {
        if seen >= units {
            return at;
        }
        seen += c.len_utf16();
    }
    text.len()
}

/// The UTF-16 range of the bytes `range` of `text`.
pub fn units(text: &str, range: &Range<usize>) -> NSRange {
    let before = text.get(..range.start).map_or(0, |s| s.encode_utf16().count());
    let inside = text.get(range.clone()).map_or(0, |s| s.encode_utf16().count());
    NSRange::new(before, inside)
}

/// The field as the picker's rules read it: its text, and the selection in bytes.
pub fn field_of(text: &str, selection: NSRange) -> Field {
    let caret = bytes(text, selection.location);
    let end = bytes(text, selection.location + selection.length);
    Field { text: text.to_string(), caret, selected: end - caret }
}

/// What a command of the field's text view is, for the keys the picker may take.
fn key_of(command: Sel) -> Option<Key> {
    Some(if command == sel!(moveUp:) {
        Key::Up
    } else if command == sel!(moveDown:) {
        Key::Down
    } else if command == sel!(pageUp:) {
        Key::PageUp
    } else if command == sel!(pageDown:) {
        Key::PageDown
    } else if command == sel!(insertNewline:) {
        Key::Enter
    } else if command == sel!(insertTab:) {
        Key::Tab
    } else if command == sel!(cancelOperation:) {
        Key::Escape
    } else if command == sel!(moveRight:) {
        Key::Right
    } else if command == sel!(moveLeft:) {
        Key::Left
    } else if command == sel!(deleteToBeginningOfLine:) {
        // ⌘⌫, which the picker takes only for a row that can be forgotten.
        Key::Forget
    } else {
        return None;
    })
}

pub struct TargetIvars {
    tx: Events,
    /// What the keys depend on, as the model last said.
    facts: Cell<Facts>,
    field: RefCell<Option<Retained<NSSearchField>>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and we don't implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholePickerTarget"]
    #[ivars = TargetIvars]
    pub struct Target;

    impl Target {
        #[unsafe(method(crumb:))]
        fn crumb(&self, sender: &NSButton) {
            self.send(Event::Crumb(sender.tag().max(0) as usize));
        }

        // The search field's own action: the cancel button clears it without a text-change notice.
        #[unsafe(method(searched:))]
        fn searched(&self, _sender: &NSSearchField) {
            self.typed();
        }
    }

    unsafe impl NSObjectProtocol for Target {}

    unsafe impl NSControlTextEditingDelegate for Target {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            self.typed();
        }

        // The keys of the field's text view, before the text view does what they do.
        #[unsafe(method(control:textView:doCommandBySelector:))]
        #[allow(non_snake_case)]
        unsafe fn control_textView_doCommandBySelector(
            &self,
            _control: &NSControl,
            text_view: &NSTextView,
            command: Sel,
        ) -> bool {
            self.command(text_view, command)
        }
    }

    unsafe impl NSTextFieldDelegate for Target {}

    unsafe impl NSWindowDelegate for Target {
        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _notification: &NSNotification) {
            self.send(Event::Resigned);
        }
    }
);

impl Target {
    #[allow(unsafe_code)]
    pub fn new(mtm: MainThreadMarker, tx: Events) -> Retained<Self> {
        let ivars = TargetIvars { tx, facts: Cell::new(Facts::default()), field: RefCell::new(None) };
        let this = Self::alloc(mtm).set_ivars(ivars);
        // SAFETY: `-[NSObject init]` is the designated initializer of the superclass.
        unsafe { msg_send![super(this), init] }
    }

    /// The field whose text this reports.
    pub fn watch(&self, field: &Retained<NSSearchField>) {
        self.ivars().field.replace(Some(field.clone()));
    }

    /// What the keys depend on, from now on.
    pub fn set_facts(&self, facts: Facts) {
        self.ivars().facts.set(facts);
    }

    /// Never blocks and never runs app code: the receiver is GPUI's task.
    fn send(&self, event: Event) {
        self.ivars().tx.try_send(event).ok();
    }

    /// Whether the picker takes `command` of the field's text view (and tells the app) instead of the field.
    fn command(&self, text_view: &NSTextView, command: Sel) -> bool {
        let Some(key) = key_of(command) else { return false };
        let field = field_of(&text_view.string().to_string(), text_view.selectedRange());
        if decide(key, &field, &self.ivars().facts.get()) == Do::Pass {
            return false;
        }
        self.send(Event::Key(key, field));
        true
    }

    /// The field's text and selection as they are now.
    fn typed(&self) {
        let Some(field) = self.ivars().field.borrow().clone() else { return };
        let text = field.stringValue().to_string();
        let selection = field
            .currentEditor()
            .map_or(NSRange::new(text.encode_utf16().count(), 0), |editor| editor.selectedRange());
        self.send(Event::Typed(field_of(&text, selection)));
    }
}

define_class!(
    // SAFETY: NSPanel may be subclassed; we only answer two questions about key status and don't implement `Drop`.
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholePickerPanel"]
    pub struct PickerPanel;

    impl PickerPanel {
        // A borderless panel can't be the key window unless it says so: the field needs the keys.
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        // And it is never the main window: the app's window stays the one the app is about.
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            false
        }
    }
);

impl PickerPanel {
    #[allow(unsafe_code)]
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: `-[NSPanel initWithContentRect:styleMask:backing:defer:]` is the designated initializer.
        unsafe {
            msg_send![
                super(this),
                initWithContentRect: NSRect::ZERO,
                styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ]
        }
    }
}

pub struct RowIvars {
    tx: Events,
    /// Which line of the list this view is showing.
    at: Cell<usize>,
}

define_class!(
    // SAFETY: NSView may be subclassed; we override the mouse and VoiceOver's press, keep our state in an
    // ivar and don't implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholePickerRow"]
    #[ivars = RowIvars]
    pub struct RowView;

    impl RowView {
        // Rows are laid out from the top.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        // The press is ours until the button comes up; the field keeps the keyboard throughout, since this view
        // doesn't ask for it.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            self.press();
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            self.press();
            true
        }
    }

    unsafe impl NSObjectProtocol for RowView {}
);

impl RowView {
    #[allow(unsafe_code)]
    pub fn new(mtm: MainThreadMarker, tx: Events) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RowIvars { tx, at: Cell::new(0) });
        // SAFETY: `-[NSView initWithFrame:]` is the designated initializer of the superclass.
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
        view.setAccessibilityElement(true);
        view
    }

    pub fn show(&self, at: usize) {
        self.ivars().at.set(at);
    }

    fn press(&self) {
        self.ivars().tx.try_send(Event::Row(self.ivars().at.get())).ok();
    }
}

pub struct WheelIvars {
    tx: Events,
    /// Scrolling not yet a whole line.
    carried: Cell<f64>,
}

define_class!(
    // SAFETY: NSView may be subclassed; we override the wheel, keep our state in an ivar and don't implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholePickerWheel"]
    #[ivars = WheelIvars]
    pub struct WheelView;

    impl WheelView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        // A trackpad moves by points and a wheel by lines; either way the list moves whole lines, and the rest
        // waits for the next event. The deltas are as the person set natural scrolling to, so "down" is up the list.
        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let delta = event.scrollingDeltaY();
            let lines = if event.hasPreciseScrollingDeltas() { delta / f64::from(ROW) } else { delta };
            let total = self.ivars().carried.get() - lines;
            let whole = total.trunc();
            self.ivars().carried.set(total - whole);
            if whole != 0.0 {
                self.ivars().tx.try_send(Event::Scroll(whole as isize)).ok();
            }
        }
    }

    unsafe impl NSObjectProtocol for WheelView {}
);

impl WheelView {
    #[allow(unsafe_code)]
    pub fn new(mtm: MainThreadMarker, tx: Events) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(WheelIvars { tx, carried: Cell::new(0.0) });
        // SAFETY: `-[NSView initWithFrame:]` is the designated initializer of the superclass.
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

#[cfg(test)]
mod tests {
    use super::{bytes, field_of, units};
    use objc2_foundation::NSRange;

    #[test]
    fn offsets_in_utf16_units_become_byte_offsets() {
        let text = "aé😀b";
        assert_eq!(bytes(text, 0), 0);
        assert_eq!(bytes(text, 1), 1);
        assert_eq!(bytes(text, 2), 3, "after é");
        assert_eq!(bytes(text, 4), 7, "after the emoji, which is two units");
        assert_eq!(bytes(text, 3), 7, "inside a pair: to its end");
        assert_eq!(bytes(text, 99), text.len());
    }

    #[test]
    fn a_byte_range_becomes_a_range_of_units() {
        let text = "aé😀b";
        let r = units(text, &(1..7));
        assert_eq!((r.location, r.length), (1, 3));
    }

    #[test]
    fn the_field_reads_the_selection_in_bytes() {
        let f = field_of("src/é", NSRange::new(4, 1));
        assert_eq!((f.caret, f.selected), (4, 2));
        let end = field_of("src/", NSRange::new(4, 0));
        assert_eq!((end.caret, end.selected), (4, 0));
    }
}
