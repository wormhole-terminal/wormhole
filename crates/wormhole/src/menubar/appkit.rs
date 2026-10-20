//! Renders a [`MenuModel`] into `NSStatusItem` + `NSMenu`: native, so VoiceOver,
//! type-to-select, Spaces, full screen and the bar's light and dark come for free. The rows'
//! glyphs are palette SF Symbols in the theme's status colours, not the system's.

use smol::channel::Sender;
use std::cell::{Cell, RefCell};
use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSCellImagePosition, NSColor, NSFont, NSImage, NSImageSymbolConfiguration, NSImageSymbolScale, NSMenu,
    NSMenuItem, NSMutableParagraphStyle, NSStatusBar, NSStatusItem, NSTextAlignment, NSTextTab,
    NSVariableStatusItemLength,
};
use objc2_foundation::{NSArray, NSDictionary, NSMutableAttributedString, NSRange, NSString};

use super::model::{self, Entry, Glyph, ItemModel, StatusIcon};
use super::{MenuCommand, MenuSnapshot, StatusColors};
use crate::native::macos::appkit::{modifier_mask, ns_color, symbol};
use target::MenuTarget;
use wormhole_core::notify::ItemState;

/// Tags carry `generation << TAG_SHIFT | index`, so a click delivered after a rebuild still
/// resolves against the menu the user actually saw (or is dropped), never a shifted index.
const TAG_SHIFT: u32 = 24;
const TAG_INDEX_MASK: isize = (1 << TAG_SHIFT) - 1;
/// The count beside the glyph: 13 / 500, digits of one width so neighbours don't shift.
const COUNT_SIZE: f64 = 13.0;
const COUNT_WEIGHT: f64 = 0.23;
/// A row's time is 12 pt, right-aligned on a tab stop past the longest title.
const TRAILING_SIZE: f64 = 12.0;

pub struct TargetIvars {
    tx: Sender<MenuCommand>,
    generation: Cell<isize>,
    /// Commands of the current and the previous generation.
    commands: RefCell<[(isize, Vec<MenuCommand>); 2]>,
    /// What the menu is built from as it opens, so its times are current then.
    snapshot: RefCell<MenuSnapshot>,
}

#[allow(unsafe_code)] // `define_class!` declares an Objective-C class; objc2 checks the signatures.
mod target {
    use objc2::rc::Retained;
    use objc2::runtime::NSObjectProtocol;
    use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
    use objc2_app_kit::{NSMenu, NSMenuDelegate, NSMenuItem};
    use objc2_foundation::NSObject;

    use super::TargetIvars;

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and we don't implement `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "WormholeMenuBarTarget"]
        #[ivars = TargetIvars]
        pub struct MenuTarget;

        impl MenuTarget {
            #[unsafe(method(menuItemClicked:))]
            fn menu_item_clicked(&self, sender: &NSMenuItem) {
                self.dispatch(sender.tag());
            }
        }

        unsafe impl NSObjectProtocol for MenuTarget {}

        unsafe impl NSMenuDelegate for MenuTarget {
            // Built as it opens: the times ("2m", "4:12") are as of now, and a closed menu costs
            // nothing however often the terminals change.
            #[unsafe(method(menuNeedsUpdate:))]
            fn menu_needs_update(&self, menu: &NSMenu) {
                super::rebuild(self.mtm(), menu, self);
            }
        }
    );

    impl MenuTarget {
        pub fn new(mtm: MainThreadMarker, ivars: TargetIvars) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(ivars);
            // SAFETY: `-[NSObject init]` is the designated initializer of the superclass.
            unsafe { msg_send![super(this), init] }
        }
    }
}

impl MenuTarget {
    fn dispatch(&self, tag: isize) {
        let ivars = self.ivars();
        let (generation, index) = (tag >> TAG_SHIFT, (tag & TAG_INDEX_MASK) as usize);
        let command = ivars
            .commands
            .borrow()
            .iter()
            .find(|(g, _)| *g == generation)
            .and_then(|(_, list)| list.get(index).cloned());
        match command {
            Some(command) => {
                tracing::debug!(?command, "menu bar command");
                if ivars.tx.try_send(command).is_err() {
                    tracing::warn!("menu bar command receiver dropped");
                }
            }
            None => tracing::debug!(tag, "stale menu bar item clicked"),
        }
    }

    fn ivars(&self) -> &TargetIvars {
        objc2::DefinedClass::ivars(self)
    }

    /// Starts a new generation (keeping the previous one for late clicks) and returns it.
    fn begin_generation(&self) -> isize {
        let ivars = self.ivars();
        let generation = (ivars.generation.get() + 1) & 0x7f;
        ivars.generation.set(generation);
        let mut commands = ivars.commands.borrow_mut();
        commands.swap(0, 1);
        commands[0] = (generation, Vec::new());
        generation
    }

    fn register(&self, generation: isize, command: MenuCommand) -> isize {
        let mut commands = self.ivars().commands.borrow_mut();
        let list = &mut commands[0].1;
        list.push(command);
        (generation << TAG_SHIFT) | (list.len() as isize - 1)
    }
}

/// Owns the status item. Dropping it removes the item from the menu bar.
pub struct MenuBar {
    mtm: MainThreadMarker,
    item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    target: Retained<MenuTarget>,
    /// The icon as last drawn, and the animation frame it was drawn at.
    icon: Option<(StatusIcon, Option<usize>, StatusColors)>,
}

impl MenuBar {
    /// Create the status item (main thread only; returns `None` elsewhere). Commands are sent
    /// on `tx` from the main thread when the user picks a menu item.
    pub fn install(tx: Sender<MenuCommand>) -> Option<Self> {
        let Some(mtm) = MainThreadMarker::new() else {
            tracing::warn!("MenuBar::install called off the main thread");
            return None;
        };
        let target = MenuTarget::new(
            mtm,
            TargetIvars {
                tx,
                generation: Cell::new(0),
                commands: RefCell::new([(-1, Vec::new()), (-1, Vec::new())]),
                snapshot: RefCell::new(MenuSnapshot::default()),
            },
        );
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        menu.setDelegate(Some(ProtocolObject::from_ref(&*target)));

        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        // Lets macOS remember where the user ⌘-dragged the item.
        item.setAutosaveName(Some(&NSString::from_str("WormholeStatusItem")));
        item.setMenu(Some(&menu));

        let mut bar = Self { mtm, item, menu, target, icon: None };
        bar.update(&MenuSnapshot::default());
        Some(bar)
    }

    /// Keep `snapshot` for the next time the menu opens, and redraw the icon if it changed.
    pub fn update(&mut self, snapshot: &MenuSnapshot) {
        let icon = model::status_icon(&snapshot.terminals, snapshot.icon, snapshot.animate);
        let frame = self.icon.as_ref().and_then(|(_, frame, _)| *frame).filter(|_| icon.animate);
        self.apply_icon(icon, frame, snapshot.colors);
        self.target.ivars().snapshot.replace(snapshot.clone());
    }

    /// Draw the working animation's `frame` (`None`: still).
    pub fn show_frame(&mut self, frame: Option<usize>) {
        if let Some((icon, _, colors)) = self.icon.clone() {
            self.apply_icon(icon, frame, colors);
        }
    }

    fn apply_icon(&mut self, icon: StatusIcon, frame: Option<usize>, colors: StatusColors) {
        let next = (icon, frame, colors);
        if self.icon.as_ref() == Some(&next) {
            return;
        }
        let (icon, frame, colors) = &next;
        if let Some(button) = self.item.button(self.mtm) {
            let filled = icon.state == ItemState::Working;
            let image = super::icon::image(filled, *frame, icon.mark, *colors, &icon.tooltip);
            button.setImage(Some(&image));
            let title = icon.count.clone().unwrap_or_default();
            button.setTitle(&NSString::from_str(&title));
            if !title.is_empty() {
                button
                    .setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(COUNT_SIZE, COUNT_WEIGHT)));
            }
            let position = if title.is_empty() {
                NSCellImagePosition::ImageOnly
            } else {
                NSCellImagePosition::ImageLeft
            };
            button.setImagePosition(position);
            button.setToolTip(Some(&NSString::from_str(&icon.tooltip)));
        }
        self.icon = Some(next);
    }
}

/// devctl: the icon's states drawn to TIFFs, in these status colours.
#[cfg(feature = "devctl")]
pub fn save_icons(
    dir: &std::path::Path,
    prefix: &str,
    colors: StatusColors,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    super::icon::save_all(dir, prefix, colors)
}

impl Drop for MenuBar {
    fn drop(&mut self) {
        self.menu.setDelegate(None);
        NSStatusBar::systemStatusBar().removeStatusItem(&self.item);
    }
}

/// Build the menu from the latest snapshot, as of now.
fn rebuild(mtm: MainThreadMarker, menu: &NSMenu, target: &MenuTarget) {
    let snapshot = target.ivars().snapshot.borrow().clone();
    let model = model::build(&snapshot, Instant::now());
    let generation = target.begin_generation();
    menu.removeAllItems();
    fill(mtm, menu, target, generation, &model.entries, snapshot.colors);
}

/// A row's glyph: the sidebar's shapes as SF Symbols, in the theme's colours (an idle or ended
/// terminal in the system's secondary label colour).
fn glyph_image(glyph: Glyph, colors: StatusColors) -> Option<Retained<NSImage>> {
    let (names, color, description): (&[&str], _, _) = match glyph {
        Glyph::NeedsYou => (&["circle.fill"], ns_color(colors.attention), "Needs you"),
        Glyph::Bell => (&["bell.fill"], ns_color(colors.attention), "Bell"),
        Glyph::Working => (&["progress.indicator", "circle.dotted"], ns_color(colors.working), "Working"),
        Glyph::Running => (&["progress.indicator", "circle.dotted"], ns_color(colors.running), "Running"),
        Glyph::Done => (&["checkmark"], ns_color(colors.done), "Finished"),
        Glyph::Failed => (&["xmark.circle", "exclamationmark.circle"], ns_color(colors.danger), "Failed"),
        Glyph::Idle => (&["circle"], NSColor::secondaryLabelColor(), "Idle"),
        Glyph::Exited => (&["minus.circle"], NSColor::secondaryLabelColor(), "Exited"),
    };
    let palette =
        NSImageSymbolConfiguration::configurationWithPaletteColors(&NSArray::from_retained_slice(&[color]));
    let config = palette.configurationByApplyingConfiguration(
        &NSImageSymbolConfiguration::configurationWithScale(NSImageSymbolScale::Small),
    );
    names.iter().find_map(|name| symbol(name, description))?.imageWithSymbolConfiguration(&config)
}

#[allow(unsafe_code)]
fn set_action(item: &NSMenuItem, target: &MenuTarget) {
    // SAFETY: `menuItemClicked:` is defined on `MenuTarget` with the `(id)sender` signature
    // AppKit uses for actions. The target is weak in AppKit; `MenuBar` keeps it alive.
    unsafe {
        item.setTarget(Some(target));
        item.setAction(Some(sel!(menuItemClicked:)));
    }
}

/// Where a menu's right-aligned times go: past its longest title (an estimate from the menu
/// font's average width, which is all a tab stop needs).
fn tab_stop(entries: &[Entry]) -> f64 {
    const CHAR_W: f64 = 7.5;
    const GAP: f64 = 48.0;
    let longest = entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Item(item) if item.trailing.is_some() => Some(item.title.chars().count()),
            Entry::Submenu { title, trailing: Some(_), .. } => Some(title.chars().count()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    (longest as f64 * CHAR_W + GAP).clamp(160.0, 360.0)
}

/// "title⇥trailing", the trailing part right-aligned at `stop` in the secondary label colour.
#[allow(unsafe_code)]
fn title_with_trailing(title: &str, trailing: &str, stop: f64) -> Retained<NSMutableAttributedString> {
    use objc2_app_kit::{NSFontAttributeName, NSForegroundColorAttributeName, NSParagraphStyleAttributeName};

    let text = format!("{title}\t{trailing}");
    let string = NSMutableAttributedString::initWithString(
        NSMutableAttributedString::alloc(),
        &NSString::from_str(&text),
    );
    let whole = NSRange::new(0, text.encode_utf16().count());
    let tail = NSRange::new(title.encode_utf16().count() + 1, trailing.encode_utf16().count());
    let style = NSMutableParagraphStyle::new();
    // NSTextAlignmentRight is 1 on macOS (objc2 leaves the platform-dependent value out).
    let right = NSTextAlignment(1);
    // SAFETY: an empty options dictionary is valid for any key and value type.
    let tab = unsafe {
        NSTextTab::initWithTextAlignment_location_options(
            NSTextTab::alloc(),
            right,
            stop,
            &NSDictionary::new(),
        )
    };
    style.setTabStops(Some(&NSArray::from_retained_slice(&[tab])));
    let menu_font = NSFont::menuFontOfSize(0.0);
    let small = NSFont::monospacedDigitSystemFontOfSize_weight(TRAILING_SIZE, 0.0);
    let muted = NSColor::secondaryLabelColor();
    // SAFETY: each value is of the type its attribute key requires (a paragraph style, a font, a
    // colour), the keys are AppKit's constants, and both ranges lie within the string.
    unsafe {
        string.addAttribute_value_range(NSParagraphStyleAttributeName, as_object(&*style), whole);
        string.addAttribute_value_range(NSFontAttributeName, as_object(&*menu_font), whole);
        string.addAttribute_value_range(NSFontAttributeName, as_object(&*small), tail);
        string.addAttribute_value_range(NSForegroundColorAttributeName, as_object(&*muted), tail);
    }
    string
}

/// Every Objective-C object is an `AnyObject`.
fn as_object<T: AsRef<AnyObject>>(value: &T) -> &AnyObject {
    value.as_ref()
}

fn menu_item(
    mtm: MainThreadMarker,
    target: &MenuTarget,
    generation: isize,
    model: &ItemModel,
    colors: StatusColors,
    stop: f64,
) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str(&model.title));
    if let Some(trailing) = &model.trailing {
        item.setAttributedTitle(Some(&title_with_trailing(&model.title, trailing, stop)));
    }
    if let Some(subtitle) = &model.subtitle {
        let subtitle = NSString::from_str(subtitle);
        item.setSubtitle(Some(&subtitle));
        item.setToolTip(Some(&subtitle));
    }
    if let Some(glyph) = model.glyph {
        item.setImage(glyph_image(glyph, colors).as_deref());
    }
    if let Some(key) = &model.key {
        item.setKeyEquivalent(&NSString::from_str(&key.key));
        item.setKeyEquivalentModifierMask(modifier_mask(key));
    }
    item.setIndentationLevel(model.indent);
    match &model.command {
        Some(command) => {
            item.setTag(target.register(generation, command.clone()));
            set_action(&item, target);
        }
        None => item.setEnabled(false),
    }
    item
}

fn fill(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    target: &MenuTarget,
    generation: isize,
    entries: &[Entry],
    colors: StatusColors,
) {
    let stop = tab_stop(entries);
    for entry in entries {
        let item = match entry {
            // macOS 14 is the minimum: native section headers, no fallback.
            Entry::Header(title) => NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(title), mtm),
            Entry::Separator => NSMenuItem::separatorItem(mtm),
            Entry::Item(model) => menu_item(mtm, target, generation, model, colors, stop),
            Entry::Submenu { title, trailing, entries } => {
                let ns_title = NSString::from_str(title);
                let submenu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &ns_title);
                submenu.setAutoenablesItems(false);
                fill(mtm, &submenu, target, generation, entries, colors);
                let item = NSMenuItem::new(mtm);
                item.setTitle(&ns_title);
                if let Some(trailing) = trailing {
                    item.setAttributedTitle(Some(&title_with_trailing(title, trailing, stop)));
                }
                // The parent item retains the submenu; dropping our handle is fine.
                item.setSubmenu(Some(&submenu));
                item
            }
        };
        menu.addItem(&item);
    }
}
