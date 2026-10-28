//! A [`Popover`] as an `NSPopover` of AppKit's own controls: an `NSTextField` with focus, bordered
//! `NSButton`s, the colour dots of `swatches`, menu-like rows that light up under the pointer. The
//! theme's light or dark is the popover's, and its arrow points at the element's bounds.
//!
//! A popover is not modal, so AppKit calls its controls' actions from its own event dispatch,
//! while GPUI may have its `App` borrowed. Nothing here runs an app callback from an action: an
//! action only sends an [`Event`] down a channel (never full, never waiting). One foreground task
//! of GPUI, started with the popover, receives them and runs the callback in an update of the
//! popover's window with nothing borrowed, the way `menu` runs the item chosen once its loop has
//! returned. The task also does what makes AppKit call GPUI back (showing and closing the popover
//! change the key window), outside every borrow. When the popover is gone it ends, and so does the
//! channel's sender with it.

use std::rc::Rc;

use gpui_kit::*;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject, Sel};
use objc2::{
    AnyThread as _, DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use objc2_app_kit::{
    NSAccessibility as _, NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua,
    NSAppearanceNameDarkAqua, NSBezelStyle, NSBox, NSBoxType, NSButton, NSCellImagePosition, NSControl,
    NSControlSize, NSControlTextEditingDelegate, NSEvent, NSFont, NSLayoutAttribute, NSLineBreakMode,
    NSPopover, NSPopoverBehavior, NSPopoverDelegate, NSStackView, NSTextField, NSTextFieldDelegate,
    NSTextView, NSTrackingArea, NSTrackingAreaOptions, NSUserInterfaceLayoutOrientation, NSView,
    NSViewController,
};
use objc2_foundation::{NSEdgeInsets, NSNotification, NSRect, NSRectEdge, NSSize, NSString};
use wormhole_core::theme::tokens::Appearance;

use super::{appkit, rows, swatches};
use crate::platform::menu::Callback;
use crate::platform::popover::{Behavior, Button, Colours, Done, Field, Glyph, Handle, Popover, Row};

/// The popover's padding, the gap between its rows, the least width it has, and how wide the path
/// wraps.
const PAD: f64 = 12.0;
const GAP: f64 = 8.0;
const MIN_WIDTH: f64 = 276.0;
const PATH_WIDTH: f64 = 392.0;
/// A paragraph's width: a few short lines, not a wide card.
const TEXT_WIDTH: f64 = 320.0;
/// `NSFontWeightRegular` and `NSFontWeightSemibold`, which AppKit gives as numbers from -1 to 1.
const REGULAR: f64 = 0.0;
const SEMIBOLD: f64 = 0.3;

/// What a control did, as the app's task is told.
enum Event {
    /// The button or action row of that tag.
    Press(usize),
    /// A dot of the colours row: its place, the one past the last named being the dot of one's own.
    Dot(usize),
    /// ↩ in the field.
    Commit,
    /// ⎋ in the field.
    Cancel,
    /// The pointer came onto or left the popover.
    Hover(bool),
    /// AppKit closed it (a click outside, ⎋ ...).
    Closed,
    /// The app closes it.
    Close,
}

type Tx = smol::channel::Sender<Event>;
/// Something to run in an update of the popover's window.
type Once = Box<dyn FnOnce(&mut Window, &mut App)>;

pub struct Ivars {
    tx: Tx,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and we don't implement `Drop`.
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholePopoverTarget"]
    #[ivars = Ivars]
    struct Target;

    impl Target {
        #[unsafe(method(pressed:))]
        fn pressed(&self, sender: &NSButton) {
            self.send(Event::Press(sender.tag().max(0) as usize));
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.send(Event::Hover(true));
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.send(Event::Hover(false));
        }
    }

    unsafe impl NSObjectProtocol for Target {}

    unsafe impl NSControlTextEditingDelegate for Target {
        // ↩ and ⎋ in the field are commands of its text view.
        #[unsafe(method(control:textView:doCommandBySelector:))]
        #[allow(non_snake_case)]
        unsafe fn control_textView_doCommandBySelector(
            &self,
            _control: &NSControl,
            _text_view: &NSTextView,
            command: Sel,
        ) -> bool {
            if command == sel!(insertNewline:) {
                self.send(Event::Commit);
                true
            } else if command == sel!(cancelOperation:) {
                self.send(Event::Cancel);
                true
            } else {
                false
            }
        }
    }

    unsafe impl NSTextFieldDelegate for Target {}

    unsafe impl NSPopoverDelegate for Target {
        #[unsafe(method(popoverDidClose:))]
        fn popover_did_close(&self, _notification: &NSNotification) {
            self.send(Event::Closed);
        }
    }
);

impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, tx: Tx) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { tx });
        // SAFETY: `-[NSObject init]` is the designated initializer of the superclass.
        unsafe { msg_send![super(this), init] }
    }

    /// Never blocks and never runs app code: the receiver is GPUI's task.
    fn send(&self, event: Event) {
        self.ivars().tx.try_send(event).ok();
    }
}

/// What a control does when it is pressed, by its tag.
struct Action {
    run: Callback,
    closes: bool,
    done: Option<Done>,
    button: Option<Retained<NSButton>>,
    title: String,
}

/// The views of a popover, and what the task needs to answer them.
struct Built {
    view: Retained<NSView>,
    size: NSSize,
    actions: Vec<Action>,
    field: Option<(Retained<NSTextField>, Field)>,
    colours: Option<Colours>,
}

/// Open `popover` under `anchor` (window coordinates, the element's bounds). Gives it back when
/// the window has no AppKit view to open it in, for the fallback to draw.
pub fn show(
    popover: Popover,
    anchor: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> Result<Handle, Popover> {
    let Some(view) = appkit::ns_view(window) else { return Err(popover) };
    let handle = window.window_handle();
    let (tx, rx) = smol::channel::unbounded::<Event>();
    let closer = tx.clone();
    cx.spawn(async move |cx| {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let target = Target::new(mtm, tx.clone());
        let built = build(mtm, &popover, &target, &tx);
        let ns_popover = NSPopover::new(mtm);
        ns_popover.setBehavior(match popover.behavior {
            Behavior::Transient => NSPopoverBehavior::Transient,
            Behavior::Hover => NSPopoverBehavior::ApplicationDefined,
        });
        ns_popover.setAppearance(appearance(popover.appearance).as_deref());
        let controller = NSViewController::new(mtm);
        controller.setView(&built.view);
        ns_popover.setContentViewController(Some(&controller));
        ns_popover.setContentSize(built.size);
        ns_popover.setDelegate(Some(ProtocolObject::from_ref(&*target)));
        // All of AppKit here is outside `cx.update`: showing makes the popover's window key, which
        // calls GPUI back.
        let edge = if view.isFlipped() { NSRectEdge::MaxY } else { NSRectEdge::MinY };
        ns_popover.showRelativeToRect_ofView_preferredEdge(appkit::rect(&view, anchor), &view, edge);
        #[allow(unsafe_code)]
        if let Some((field, spec)) = &built.field
            && spec.focused
            && let Some(window) = field.window()
        {
            window.makeFirstResponder(Some(field));
            // SAFETY: a plain call on a field that is in the popover's window; the sender is nil.
            unsafe { field.selectText(None) };
        }

        let run = |f: Once| {
            cx.update(|cx| {
                handle.update(cx, move |_, window, cx| f(window, cx)).ok();
            });
        };
        let mut committed = false;
        // Ends the popover: the field's text is kept unless ⎋ dropped it, and `on_close` runs.
        let finish = |ns_popover: &NSPopover, committed: bool, close: bool| {
            if close && ns_popover.isShown() {
                ns_popover.close();
            }
            let typed = built
                .field
                .as_ref()
                .filter(|_| !committed)
                .map(|(field, spec)| (field.stringValue().to_string(), spec.on_commit.clone()));
            let on_close = popover.on_close.clone();
            run(Box::new(move |window, cx| {
                if let Some((text, commit)) = typed {
                    commit(&text, window, cx);
                }
                if let Some(on_close) = on_close {
                    on_close(window, cx);
                }
            }));
        };
        while let Ok(event) = rx.recv().await {
            match event {
                Event::Press(tag) => {
                    let Some(action) = built.actions.get(tag) else { continue };
                    let on_press = action.run.clone();
                    if action.closes {
                        // The popover goes first: what the action does may take the keyboard.
                        if ns_popover.isShown() {
                            ns_popover.close();
                        }
                        run(Box::new(move |window, cx| on_press(window, cx)));
                        finish(&ns_popover, true, false);
                        break;
                    }
                    run(Box::new(move |window, cx| on_press(window, cx)));
                    if let (Some(done), Some(button)) = (action.done.clone(), action.button.clone()) {
                        let before = action.title.clone();
                        say(&button, &done.title, done.glyph);
                        cx.spawn(async move |cx| {
                            cx.background_executor().timer(done.for_).await;
                            button.setTitle(&NSString::from_str(&before));
                            button.setImage(None);
                        })
                        .detach();
                    }
                }
                Event::Dot(ix) => {
                    let Some(colours) = &built.colours else { continue };
                    if let Some(custom) =
                        colours.custom.clone().filter(|_| ix == colours.swatches.swatches.len())
                    {
                        // The panel is another window: the popover makes way for it.
                        if ns_popover.isShown() {
                            ns_popover.close();
                        }
                        run(Box::new(move |window, cx| {
                            let start = custom.color.unwrap_or(custom.start);
                            crate::platform::pick_colour(start, custom.on_change.clone(), window, cx);
                        }));
                        finish(&ns_popover, false, false);
                        break;
                    }
                    let pick = colours.swatches.on_pick.clone();
                    run(Box::new(move |window, cx| pick(ix, window, cx)));
                }
                Event::Commit => {
                    let typed = built
                        .field
                        .as_ref()
                        .map(|(field, spec)| (field.stringValue().to_string(), spec.on_commit.clone()));
                    committed = true;
                    if ns_popover.isShown() {
                        ns_popover.close();
                    }
                    if let Some((text, commit)) = typed {
                        run(Box::new(move |window, cx| commit(&text, window, cx)));
                    }
                    finish(&ns_popover, committed, false);
                    break;
                }
                Event::Cancel => {
                    let cancel = built.field.as_ref().and_then(|(_, spec)| spec.on_cancel.clone());
                    if ns_popover.isShown() {
                        ns_popover.close();
                    }
                    if let Some(cancel) = cancel {
                        run(Box::new(move |window, cx| cancel(window, cx)));
                    }
                    finish(&ns_popover, true, false);
                    break;
                }
                Event::Hover(over) => {
                    if let Some(on_hover) = popover.on_hover.clone() {
                        run(Box::new(move |window, cx| on_hover(over, window, cx)));
                    }
                }
                Event::Closed => {
                    finish(&ns_popover, committed, false);
                    break;
                }
                Event::Close => {
                    finish(&ns_popover, committed, true);
                    break;
                }
            }
        }
        // The delegate and the controls hold the sender; the receiver is gone with this task.
        drop((target, controller));
    })
    .detach();
    Ok(Handle::new(move |_| {
        closer.try_send(Event::Close).ok();
    }))
}

fn appearance(appearance: Appearance) -> Option<Retained<NSAppearance>> {
    #[allow(unsafe_code)]
    // SAFETY: AppKit's appearance names are immutable statics, set before main runs.
    let name = unsafe {
        match appearance {
            Appearance::Light => NSAppearanceNameAqua,
            Appearance::Dark => NSAppearanceNameDarkAqua,
        }
    };
    NSAppearance::appearanceNamed(name)
}

/// The popover's controls: a vertical stack of its rows.
fn build(mtm: MainThreadMarker, popover: &Popover, target: &Target, tx: &Tx) -> Built {
    let stack = NSStackView::new(mtm);
    stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    // The rows that should reach both edges are told to (`fill`).
    stack.setAlignment(NSLayoutAttribute::Leading);
    stack.setSpacing(GAP);
    stack.setEdgeInsets(NSEdgeInsets { top: PAD, left: PAD, bottom: PAD, right: PAD });
    let mut actions: Vec<Action> = Vec::new();
    let mut field = None;
    let mut colours = None;
    // The paragraphs wrap at their own width, which the card must have.
    let mut least = MIN_WIDTH;
    for row in &popover.rows {
        match row {
            Row::Heading(text) => {
                let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
                label.setFont(Some(&NSFont::systemFontOfSize_weight(NSFont::systemFontSize(), SEMIBOLD)));
                label.setPreferredMaxLayoutWidth(TEXT_WIDTH);
                label.setSelectable(true);
                stack.addArrangedSubview(&label);
                least = least.max(TEXT_WIDTH + 2.0 * PAD);
            }
            Row::Text(text) => {
                let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
                label.setFont(Some(&NSFont::systemFontOfSize(NSFont::smallSystemFontSize())));
                label.setPreferredMaxLayoutWidth(TEXT_WIDTH);
                label.setSelectable(true);
                stack.addArrangedSubview(&label);
                least = least.max(TEXT_WIDTH + 2.0 * PAD);
            }
            Row::Code(text) => {
                let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
                label.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(
                    NSFont::smallSystemFontSize(),
                    REGULAR,
                )));
                label.setPreferredMaxLayoutWidth(TEXT_WIDTH);
                label.setLineBreakMode(NSLineBreakMode::ByCharWrapping);
                label.setSelectable(true);
                stack.addArrangedSubview(&label);
                least = least.max(TEXT_WIDTH + 2.0 * PAD);
            }
            Row::Path(text) => {
                let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
                label.setPreferredMaxLayoutWidth(PATH_WIDTH);
                label.setLineBreakMode(NSLineBreakMode::ByCharWrapping);
                label.setSelectable(true);
                stack.addArrangedSubview(&label);
            }
            Row::Field(spec) => {
                let input = NSTextField::textFieldWithString(&NSString::from_str(&spec.value), mtm);
                input.setPlaceholderString(Some(&NSString::from_str(&spec.label)));
                // SAFETY: the delegate is weak; the popover's task keeps the target alive.
                #[allow(unsafe_code)]
                unsafe {
                    input.setDelegate(Some(ProtocolObject::from_ref(target)));
                }
                input.setAccessibilityLabel(Some(&NSString::from_str(&spec.label)));
                stack.addArrangedSubview(&input);
                fill(&input, &stack);
                field = Some((input, spec.clone()));
            }
            Row::Buttons(buttons) => {
                let line = NSStackView::new(mtm);
                line.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
                line.setSpacing(6.0);
                for button in buttons {
                    let ns_button = button_for(mtm, button, target, actions.len());
                    actions.push(Action {
                        run: button.on_press.clone(),
                        closes: button.closes,
                        done: button.done.clone(),
                        button: Some(ns_button.clone()),
                        title: button.title.to_string(),
                    });
                    line.addArrangedSubview(&ns_button);
                }
                stack.addArrangedSubview(&line);
            }
            Row::Colours(spec) => {
                let tx = tx.clone();
                let how = swatches::Use {
                    inset: 0.0,
                    notify: Some(Rc::new(move |ix| {
                        tx.try_send(Event::Dot(ix)).ok();
                    })),
                    custom: spec.custom.as_ref().map(|custom| {
                        custom.color.map(|color| {
                            let rgba = Rgba::from(color);
                            [rgba.r, rgba.g, rgba.b, 1.0]
                        })
                    }),
                };
                let size = swatches::row_size(&spec.swatches, spec.custom.is_some(), 0.0);
                let row_view = swatches::row_view(mtm, 0, &spec.swatches, &Rc::default(), how);
                fix_height(&row_view, size.height);
                // The row takes the width, and its dots stay at the left, at their own places.
                stack.addArrangedSubview(&row_view);
                fill(&row_view, &stack);
                colours = Some(spec.clone());
            }
            Row::Actions(items) => {
                for item in items {
                    let sender = tx.clone();
                    let ns_button = rows::action_row(
                        mtm,
                        &item.title,
                        item.enabled,
                        actions.len(),
                        Rc::new(move |tag| {
                            sender.try_send(Event::Press(tag)).ok();
                        }),
                    );
                    actions.push(Action {
                        run: item.on_select.clone().unwrap_or_else(|| Rc::new(|_, _| {})),
                        closes: true,
                        done: None,
                        button: None,
                        title: item.title.to_string(),
                    });
                    stack.addArrangedSubview(&ns_button);
                    fill(&ns_button, &stack);
                }
            }
            Row::Separator => {
                let rule = NSBox::new(mtm);
                rule.setBoxType(NSBoxType::Separator);
                stack.addArrangedSubview(&rule);
                fill(&rule, &stack);
            }
        }
    }
    if popover.on_hover.is_some() {
        // The area follows the stack's bounds, so it needs no layout code.
        #[allow(unsafe_code)]
        // SAFETY: the owner is the target, which implements `mouseEntered:` and `mouseExited:` and
        // outlives the view (the task holds it until the popover is gone).
        let area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                NSRect::ZERO,
                NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect,
                Some(target),
                None,
            )
        };
        stack.addTrackingArea(&area);
    }
    min_width(&stack, least);
    stack.layoutSubtreeIfNeeded();
    let size = stack.fittingSize();
    stack.setFrameSize(size);
    stack.setAppearance(appearance(popover.appearance).as_deref());
    Built { view: Retained::into_super(stack), size, actions, field, colours }
}

/// A bordered push button: the title, a small picture, the control size of a popover.
fn button_for(mtm: MainThreadMarker, button: &Button, target: &Target, tag: usize) -> Retained<NSButton> {
    #[allow(unsafe_code)]
    // SAFETY: `pressed:` is defined on `Target` with the `(id)sender` signature AppKit uses for
    // actions; the popover's task keeps the target alive, and AppKit holds it weakly.
    let ns_button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(&button.title),
            Some(target),
            Some(sel!(pressed:)),
            mtm,
        )
    };
    ns_button.setBezelStyle(NSBezelStyle::Push);
    ns_button.setControlSize(NSControlSize::Small);
    ns_button.setTag(tag as isize);
    ns_button.setEnabled(button.enabled);
    if let Some(hint) = &button.hint {
        ns_button.setToolTip(Some(&NSString::from_str(hint)));
    }
    if let Some(glyph) = button.glyph {
        ns_button.setImage(symbol(glyph).as_deref());
        ns_button.setImagePosition(NSCellImagePosition::ImageLeading);
    }
    ns_button
}

fn say(button: &NSButton, title: &str, glyph: Option<Glyph>) {
    button.setTitle(&NSString::from_str(title));
    button.setImage(glyph.and_then(symbol).as_deref());
}

fn symbol(glyph: Glyph) -> Option<Retained<objc2_app_kit::NSImage>> {
    let (name, description) = match glyph {
        Glyph::Folder => ("folder", "Folder"),
        Glyph::Copy => ("doc.on.doc", "Copy"),
        Glyph::Check => ("checkmark", "Done"),
        Glyph::Terminal => ("terminal", "Terminal"),
    };
    appkit::symbol(name, description)
}

/// `view` is as wide as the stack's content: its edges, less the padding.
fn fill(view: &NSView, stack: &NSStackView) {
    view.setTranslatesAutoresizingMaskIntoConstraints(false);
    view.widthAnchor().constraintEqualToAnchor_constant(&stack.widthAnchor(), -2.0 * PAD).setActive(true);
}

/// `view` is `height` tall: a plain view has no size of its own for a stack to lay out.
fn fix_height(view: &NSView, height: f64) {
    view.setTranslatesAutoresizingMaskIntoConstraints(false);
    view.heightAnchor().constraintEqualToConstant(height).setActive(true);
}

fn min_width(view: &NSView, width: f64) {
    view.widthAnchor().constraintGreaterThanOrEqualToConstant(width).setActive(true);
}

/// devctl: build `popover` as the real controls without showing it (a background instance must
/// not take the keys) and draw them into a PNG, as AppKit lays them out. Returns the size in
/// points and what each control is. `None` off the main thread.
#[cfg(any(test, feature = "devctl"))]
#[allow(unsafe_code)]
pub fn render(popover: &Popover, png: &std::path::Path) -> Option<String> {
    use objc2_app_kit::NSBitmapImageFileType;
    use objc2_foundation::NSDictionary;
    use wormhole_core::storage::files::{self, LinkPolicy};

    let mtm = MainThreadMarker::new()?;
    let (tx, _rx) = smol::channel::unbounded::<Event>();
    let target = Target::new(mtm, tx.clone());
    let built = build(mtm, popover, &target, &tx);
    // On the popover's own background: the controls are drawn for it, not for white paper.
    let backdrop = NSBox::new(mtm);
    backdrop.setBoxType(NSBoxType::Custom);
    backdrop.setBorderWidth(0.0);
    backdrop.setFillColor(&objc2_app_kit::NSColor::windowBackgroundColor());
    backdrop.setContentViewMargins(NSSize::new(0.0, 0.0));
    backdrop.setFrameSize(built.size);
    backdrop.setAppearance(appearance(popover.appearance).as_deref());
    backdrop.setContentView(Some(&built.view));
    let bounds = backdrop.bounds();
    let rep = backdrop.bitmapImageRepForCachingDisplayInRect(bounds)?;
    backdrop.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
    // SAFETY: the properties are an empty dictionary, which the PNG encoder accepts.
    let data =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
    files::write_atomic(png, &data.to_vec(), LinkPolicy::Follow).ok()?;
    let summary = format!(
        "{}x{} points, {} actions, field {}, colours {}",
        bounds.size.width,
        bounds.size.height,
        built.actions.len(),
        built.field.is_some(),
        built.colours.is_some()
    );
    Some(summary)
}
