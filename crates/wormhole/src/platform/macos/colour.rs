//! The system's Colours window (`NSColorPanel`) to choose a colour of one's own, which the app
//! follows while it is dragged.
//!
//! The panel is not modal and calls its target's action from AppKit's own event dispatch, once
//! for every step of a drag, while GPUI may have its `App` borrowed. So the action only sends the
//! colour down a channel; a foreground task of GPUI takes what has come since it last looked,
//! keeps the latest only, and runs the callback in an update of the window with nothing borrowed.

use std::cell::RefCell;

use gpui_kit::*;
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSColorPanel, NSColorSpace};

use super::appkit;
use crate::platform::popover::ColourChange;

/// The sRGB colour the panel shows now, each channel 0–1.
type Colour = [f64; 4];

thread_local! {
    /// The target the panel is wired to: replaced by the next session, which ends the last one's
    /// task (its channel closes).
    static SESSION: RefCell<Option<Retained<Target>>> = const { RefCell::new(None) };
}

pub struct Ivars {
    tx: smol::channel::Sender<Colour>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and we don't implement `Drop`.
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeColourTarget"]
    #[ivars = Ivars]
    struct Target;

    impl Target {
        #[unsafe(method(changeColor:))]
        fn change_color(&self, sender: &NSColorPanel) {
            let srgb = sender.color().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace());
            if let Some(color) = srgb {
                // Unbounded: never full, never waits. A closed receiver means the session is over.
                self.ivars().tx.try_send([color.redComponent(), color.greenComponent(), color.blueComponent(), color.alphaComponent()]).ok();
            }
        }
    }

    unsafe impl NSObjectProtocol for Target {}
);

impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, tx: smol::channel::Sender<Colour>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { tx });
        // SAFETY: `-[NSObject init]` is the designated initializer of the superclass.
        unsafe { msg_send![super(this), init] }
    }
}

/// Open the Colours window showing `start`; every colour chosen in it reaches `on_change`.
pub fn pick(start: Hsla, alpha: bool, on_change: ColourChange, window: &mut Window, cx: &mut App) {
    let handle = window.window_handle();
    let (tx, rx) = smol::channel::unbounded::<Colour>();
    cx.spawn(async move |cx| {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let target = Target::new(mtm, tx);
        let rgba = Rgba::from(start);
        let panel = NSColorPanel::sharedColorPanel(mtm);
        panel.setShowsAlpha(alpha);
        panel.setContinuous(true);
        panel.setColor(&appkit::ns_color([rgba.r, rgba.g, rgba.b, if alpha { rgba.a } else { 1.0 }]));
        // The target is wired after the colour is set: setting it is not a choice.
        #[allow(unsafe_code)]
        // SAFETY: `changeColor:` is defined on `Target` with the `(id)sender` signature AppKit uses
        // for actions; the session keeps the target alive, and AppKit holds it weakly.
        unsafe {
            panel.setTarget(Some(&target));
            panel.setAction(Some(sel!(changeColor:)));
        }
        panel.makeKeyAndOrderFront(None);
        SESSION.with_borrow_mut(|session| *session = Some(target));
        while let Ok(mut latest) = rx.recv().await {
            while let Ok(next) = rx.try_recv() {
                latest = next;
            }
            let [r, g, b, a] = latest;
            let colour: Hsla =
                Rgba { r: r as f32, g: g as f32, b: b as f32, a: if alpha { a as f32 } else { 1.0 } }.into();
            let on_change = on_change.clone();
            cx.update(|cx| {
                handle.update(cx, move |_, window, cx| on_change(colour, window, cx)).ok();
            });
        }
    })
    .detach();
}
