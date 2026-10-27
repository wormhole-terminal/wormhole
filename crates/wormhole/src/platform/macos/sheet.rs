//! AppKit's side of `platform::sheet`: an `NSAlert` begun as a sheet, its suppression button the
//! checkbox.

use block2::RcBlock;
use gpui_kit::Window;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSAlertStyle, NSControlStateValueOn, NSModalResponse};
use objc2_foundation::NSString;
use smol::channel::Receiver;

use super::appkit::ns_window;
use crate::platform::sheet::Question;

/// `NSAlertFirstButtonReturn`: buttons answer 1000 + their place in the order they were added.
const FIRST_BUTTON: isize = 1000;

pub fn ask_with_checkbox(window: &Window, question: &Question) -> Option<Receiver<Option<bool>>> {
    let mtm = MainThreadMarker::new()?;
    let ns_window = ns_window(window)?;
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Warning);
    alert.setMessageText(&NSString::from_str(question.title));
    alert.setInformativeText(&NSString::from_str(question.detail));
    // The closing button first, so Return presses it; Cancel after it, answering to Esc.
    alert.addButtonWithTitle(&NSString::from_str(question.confirm));
    alert.addButtonWithTitle(&NSString::from_str("Cancel")).setKeyEquivalent(&NSString::from_str("\u{1b}"));
    alert.setShowsSuppressionButton(true);
    if let Some(keep) = alert.suppressionButton() {
        keep.setTitle(&NSString::from_str(question.checkbox));
        keep.setState(NSControlStateValueOn);
    }

    let (tx, rx) = smol::channel::bounded(1);
    let sheet = alert.clone();
    let done = RcBlock::new(move |response: NSModalResponse| {
        let keep = sheet.suppressionButton().is_none_or(|b| b.state() == NSControlStateValueOn);
        tx.try_send((response == FIRST_BUTTON).then_some(keep)).ok();
    });
    // AppKit copies the block and keeps the alert until the sheet is answered.
    alert.beginSheetModalForWindow_completionHandler(&ns_window, Some(&done));
    Some(rx)
}

/// End the sheet on `window` with `button` as its answer, which is what AppKit hands the completion handler
/// of a button's press (GPUI's prompt sets each button's tag to its place, and answers with it).
#[cfg(feature = "devctl")]
pub fn answer(window: &Window, button: isize) -> bool {
    let Some(ns_window) = ns_window(window) else { return false };
    let Some(sheet) = ns_window.attachedSheet() else { return false };
    ns_window.endSheet_returnCode(&sheet, button);
    true
}
