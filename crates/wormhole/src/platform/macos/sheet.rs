//! AppKit's side of `platform::sheet`: an `NSAlert` begun as a sheet, its suppression button the
//! checkbox.

use block2::RcBlock;
use gpui_kit::Window;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSControlStateValueOff, NSControlStateValueOn, NSFont, NSModalResponse,
    NSTextField,
};
use objc2_foundation::NSString;
use smol::channel::Receiver;

use super::appkit::ns_window;
use crate::platform::sheet::{Choice, Question, Style};

/// `NSAlertFirstButtonReturn`: buttons answer 1000 + their place in the order they were added.
const FIRST_BUTTON: isize = 1000;
/// `NSAlertSecondButtonReturn`: the button that backs out. Any other answer is the sheet being taken away
/// (`NSModalResponseStop` and `Abort` when its window closes or the app quits), which is no answer.
const SECOND_BUTTON: isize = 1001;

pub fn ask_with_checkbox(window: &Window, question: &Question) -> Option<Receiver<Option<bool>>> {
    let mtm = MainThreadMarker::new()?;
    let ns_window = ns_window(window)?;
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(match question.style {
        Style::Warning => NSAlertStyle::Warning,
        Style::Informational => NSAlertStyle::Informational,
    });
    alert.setMessageText(&NSString::from_str(question.title));
    alert.setInformativeText(&NSString::from_str(question.detail));
    // The closing button first, so Return presses it; the one that backs out after it, answering to Esc.
    alert.addButtonWithTitle(&NSString::from_str(question.confirm));
    alert
        .addButtonWithTitle(&NSString::from_str(question.cancel))
        .setKeyEquivalent(&NSString::from_str("\u{1b}"));
    alert.setShowsSuppressionButton(true);
    if let Some(keep) = alert.suppressionButton() {
        keep.setTitle(&NSString::from_str(question.checkbox));
        keep.setState(if question.checked { NSControlStateValueOn } else { NSControlStateValueOff });
    }

    let (tx, rx) = smol::channel::bounded(1);
    let sheet = alert.clone();
    let started_ticked = question.checked;
    let done = RcBlock::new(move |response: NSModalResponse| {
        #[cfg(feature = "devctl")]
        up::end(&sheet);
        // A box that can't be read keeps the state it began with: for the question before quitting that
        // is ticked, so the terminals are never ended by accident.
        let ticked = sheet.suppressionButton().map_or(started_ticked, |b| b.state() == NSControlStateValueOn);
        match response {
            FIRST_BUTTON => tx.try_send(Some(ticked)).ok(),
            SECOND_BUTTON => tx.try_send(None).ok(),
            _ => None,
        };
    });
    #[cfg(feature = "devctl")]
    up::begin(&alert);
    // AppKit copies the block and keeps the alert until the sheet is answered.
    alert.beginSheetModalForWindow_completionHandler(&ns_window, Some(&done));
    Some(rx)
}

/// End the sheet on `window` with button number `button` (in the order they were added) as its answer, which
/// is what AppKit hands the completion handler of a button's press.
#[cfg(feature = "devctl")]
pub fn answer(window: &Window, button: isize, tick: Option<bool>) -> bool {
    let Some(ns_window) = ns_window(window) else { return false };
    let Some(sheet) = ns_window.attachedSheet() else { return false };
    // An alert answers 1000 + its button's place; GPUI's own prompt answers with the place itself.
    let code = match up::alert_of(&sheet) {
        Some(alert) => {
            if let Some(tick) = tick
                && let Some(checkbox) = alert.suppressionButton()
            {
                checkbox.setState(if tick { NSControlStateValueOn } else { NSControlStateValueOff });
            }
            // The alert has the closing button and the one that backs out; anything but the first is the latter.
            FIRST_BUTTON + isize::from(button != 0)
        }
        None => button,
    };
    ns_window.endSheet_returnCode(&sheet, code);
    true
}

/// How wide the details block is, in points: the width of a sheet's text.
const ACCESSORY_WIDTH: f64 = 380.0;
/// The size of the details' monospace text.
const ACCESSORY_FONT: f64 = 11.0;
/// `NSFontWeightRegular`: the weight 0, for a font of no weight in particular.
const REGULAR: f64 = 0.0;
/// What a sheet's completion handler gets for a sheet ended without a button.
const NO_BUTTON: NSModalResponse = 0;

pub fn choose(window: &Window, choice: &Choice) -> Option<Receiver<Option<usize>>> {
    let mtm = MainThreadMarker::new()?;
    let ns_window = ns_window(window)?;
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.setMessageText(&NSString::from_str(choice.title));
    alert.setInformativeText(&NSString::from_str(choice.detail));
    for button in choice.buttons {
        alert.addButtonWithTitle(&NSString::from_str(button));
    }
    if let Some(text) = choice.accessory {
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
        label.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(ACCESSORY_FONT, REGULAR)));
        label.setPreferredMaxLayoutWidth(ACCESSORY_WIDTH);
        label.setFrameSize(label.fittingSize());
        alert.setAccessoryView(Some(&label));
    }
    let (tx, rx) = smol::channel::bounded(1);
    let count = choice.buttons.len();
    let done = RcBlock::new(move |response: NSModalResponse| {
        let place = usize::try_from(response - FIRST_BUTTON).ok().filter(|place| *place < count);
        tx.try_send(place).ok();
    });
    alert.beginSheetModalForWindow_completionHandler(&ns_window, Some(&done));
    Some(rx)
}

pub fn dismiss(window: &Window) {
    let Some(ns_window) = ns_window(window) else { return };
    if let Some(sheet) = ns_window.attachedSheet() {
        ns_window.endSheet_returnCode(&sheet, NO_BUTTON);
    }
}

/// The alerts whose sheets are up, so `answer` can reach an alert's box from the sheet's window.
#[cfg(feature = "devctl")]
mod up {
    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2_app_kit::{NSAlert, NSWindow};

    thread_local! {
        static UP: RefCell<Vec<Retained<NSAlert>>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn begin(alert: &Retained<NSAlert>) {
        UP.with(|up| up.borrow_mut().push(alert.clone()));
    }

    pub(super) fn end(alert: &Retained<NSAlert>) {
        UP.with(|up| up.borrow_mut().retain(|other| !std::ptr::eq(&**other, &**alert)));
    }

    /// The alert shown in `sheet`.
    pub(super) fn alert_of(sheet: &NSWindow) -> Option<Retained<NSAlert>> {
        UP.with(|up| up.borrow().iter().find(|alert| std::ptr::eq(&*alert.window(), sheet)).cloned())
    }
}
