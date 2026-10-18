//! The app's questions. The one before closing a busy terminal, window or the app is the
//! platform's own alert, a sheet on the window, like the Studio's unsaved draft: macOS draws it,
//! so it looks and behaves as every other Mac alert does (Return presses the closing button, Esc
//! cancels). The rest (the new worktree's branch, a notice) are GPUI Kit's alert dialog on an
//! opaque `surface.elevated` card: the Kit paints a dialog in its window background, which the
//! bridge keeps clear so the app's own surfaces show over the blur, so a dialog of its own would
//! show whatever is behind it. Opaque under Increase Contrast too, like every surface.
//! The question before quitting with terminals in the background is an `NSAlert` of our own: GPUI's
//! prompt has no checkbox.

use std::rc::Rc;

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::dialog::AlertDialog;
use gpui_kit::prelude::*;
use gpui_kit::{App, PromptButton, PromptLevel, SharedString, Window};

use crate::settings::ui;

/// Open an alert dialog over `window`: `build` fills it in, as for the Kit's `open_alert_dialog`.
pub(crate) fn open_alert(
    window: &mut Window,
    cx: &mut App,
    build: impl Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
) {
    window.open_alert_dialog(cx, move |alert, window, cx| {
        // Read on every draw, so a theme change while the question is up recolours it.
        let card = ui(cx).elevated;
        build(alert.bg(card), window, cx)
    });
}

/// What a question before closing a terminal or a window says.
pub(crate) struct CloseQuestion {
    pub title: SharedString,
    pub detail: SharedString,
    /// The closing button's words ("Close", "Close Window").
    pub confirm: &'static str,
}

/// What a question's closing button runs.
pub(crate) type OnConfirm = Rc<dyn Fn(&mut Window, &mut App)>;

/// A second way to close, beside the closing button: its label and what it does instead.
pub(crate) type Also = (&'static str, OnConfirm);

/// The answers' places: the closing button first, so Return presses it, then the `also` button
/// when there is one, and Cancel last (AppKit lays them out right to left, Cancel at the left).
const CONFIRM: usize = 0;
const ALSO: usize = 1;

/// Ask `question` in a sheet on `window`. `on_confirm` runs for the closing button;
/// `on_dismiss` whenever the question goes away, after answering too. The window going
/// first (the app quitting) answers with nothing, as Cancel.
pub(crate) fn ask_before_closing(
    window: &mut Window,
    cx: &mut App,
    question: CloseQuestion,
    on_confirm: impl FnOnce(&mut Window, &mut App) + 'static,
    on_dismiss: impl FnOnce(&mut App) + 'static,
) {
    ask_before_closing_with(window, cx, question, None, on_confirm, on_dismiss);
}

/// [`ask_before_closing`] with an `also` button between the closing one and Cancel (it does more
/// than closing: the question before closing a terminal in a worktree Wormhole made offers to
/// remove the worktree too). Return still presses the closing button.
pub(crate) fn ask_before_closing_with(
    window: &mut Window,
    cx: &mut App,
    question: CloseQuestion,
    also: Option<Also>,
    on_confirm: impl FnOnce(&mut Window, &mut App) + 'static,
    on_dismiss: impl FnOnce(&mut App) + 'static,
) {
    ask_with_cancel(window, cx, question, "Cancel", also, on_confirm, on_dismiss);
}

/// [`ask_before_closing_with`] with the last button worded `cancel`, for a question that is about
/// more than closing: the one before restarting to update lets you say "Later". It is still the
/// button Esc presses, and it still does nothing.
pub(crate) fn ask_with_cancel(
    window: &mut Window,
    cx: &mut App,
    question: CloseQuestion,
    cancel: &'static str,
    also: Option<Also>,
    on_confirm: impl FnOnce(&mut Window, &mut App) + 'static,
    on_dismiss: impl FnOnce(&mut App) + 'static,
) {
    let mut buttons = vec![PromptButton::ok(question.confirm)];
    buttons.extend(also.as_ref().map(|(label, _)| PromptButton::new(*label)));
    buttons.push(PromptButton::cancel(cancel));
    let answer = window.prompt(PromptLevel::Warning, &question.title, Some(&question.detail), &buttons, cx);
    window
        .spawn(cx, async move |cx| {
            let answer = answer.await.ok();
            cx.update(|window, cx| {
                match (answer, also) {
                    (Some(CONFIRM), _) => on_confirm(window, cx),
                    (Some(ALSO), Some((_, on_also))) => on_also(window, cx),
                    _ => {}
                }
                on_dismiss(cx);
            })
            .ok();
        })
        .detach();
}

/// The checkbox of the question before quitting: whether the terminals in session hosts stay.
const KEEP_RUNNING: &str = "Keep terminals running in the background";

/// `NSAlertFirstButtonReturn`: buttons answer 1000 + their place in the order they were added.
#[cfg(target_os = "macos")]
const FIRST_BUTTON: isize = 1000;

/// What the quit sheet answered: `None` for Cancel (or the window going), else whether the box
/// "Keep terminals running in the background" was ticked when Quit was pressed.
type KeepAnswer = Option<bool>;

/// Ask the question before quitting in a sheet with the box [`KEEP_RUNNING`], ticked every time
/// (the choice isn't remembered: the next quit asks afresh). `on_confirm` runs for Quit with
/// the box's state; `on_dismiss` whenever the question goes away, as for [`ask_before_closing`].
/// A window that can't show the native sheet gets the plain question, which quits keeping them.
pub(crate) fn ask_keeping_terminals(
    window: &mut Window,
    cx: &mut App,
    question: CloseQuestion,
    on_confirm: impl FnOnce(bool, &mut Window, &mut App) + 'static,
    on_dismiss: impl FnOnce(&mut App) + 'static,
) {
    let Some(answer) = native_sheet(window, &question) else {
        ask_before_closing(window, cx, question, move |window, cx| on_confirm(true, window, cx), on_dismiss);
        return;
    };
    window
        .spawn(cx, async move |cx| {
            let answer = answer.recv().await.ok().flatten();
            cx.update(|window, cx| {
                if let Some(keep) = answer {
                    on_confirm(keep, window, cx);
                }
                on_dismiss(cx);
            })
            .ok();
        })
        .detach();
}

/// Show the sheet on `window`'s NSWindow; its answer arrives on the channel. `None`: no NSWindow
/// to put it on (off macOS, or a window without one).
#[cfg(target_os = "macos")]
fn native_sheet(window: &Window, question: &CloseQuestion) -> Option<smol::channel::Receiver<KeepAnswer>> {
    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertStyle, NSControlStateValueOn, NSModalResponse};
    use objc2_foundation::NSString;

    let mtm = MainThreadMarker::new()?;
    let ns_window = crate::traffic_lights::ns_window(window)?;
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Warning);
    alert.setMessageText(&NSString::from_str(&question.title));
    alert.setInformativeText(&NSString::from_str(&question.detail));
    // The closing button first, so Return presses it; Cancel after it, answering to Esc.
    alert.addButtonWithTitle(&NSString::from_str(question.confirm));
    alert.addButtonWithTitle(&NSString::from_str("Cancel")).setKeyEquivalent(&NSString::from_str("\u{1b}"));
    alert.setShowsSuppressionButton(true);
    if let Some(keep) = alert.suppressionButton() {
        keep.setTitle(&NSString::from_str(KEEP_RUNNING));
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

#[cfg(not(target_os = "macos"))]
fn native_sheet(_: &Window, _: &CloseQuestion) -> Option<smol::channel::Receiver<KeepAnswer>> {
    None
}
