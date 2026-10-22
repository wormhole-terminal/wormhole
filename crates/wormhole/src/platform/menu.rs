//! What a menu is, whoever draws it: titles, checks, keys, small pictures, separators, section
//! headers, submenus and what choosing an item does. No AppKit and no widget is named here, so
//! the same value is a native `NSMenu` on macOS (`macos::menu`), GPUI's own popup elsewhere
//! (`style::menu_layer`, handed over as `Drawn`), and plain text in tests and in devctl's `menu` command.
//!
//! The model lives in the app, not in `wormhole-core`: choosing an item runs a closure on GPUI's
//! `Window` and `App`, and core has no GPUI.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::*;

pub use crate::menubar::model::KeyEquivalent;

/// What choosing an item does. It runs after the menu has closed, outside every borrow of the app,
/// so it may update any entity or open a window.
pub type Callback = Rc<dyn Fn(&mut Window, &mut App)>;

/// What choosing a swatch does, given its place in the row. Runs like a [`Callback`].
pub type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A small picture in front of an item's title.
#[derive(Clone)]
pub enum Icon {
    /// An app's icon, as the bytes of an image file.
    Image(Arc<Image>),
    /// A file's or folder's icon as the system draws it in Finder (the house for a home folder,
    /// a disk for a volume). Nothing is read here: the platform asks for the icon.
    File(PathBuf),
    /// Two letters in a small rounded square, for an app whose icon can't be had (where there is
    /// no icon service to ask, the fallback menu draws it; the native one has the icon).
    Monogram(SharedString),
}

/// An item's check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Check {
    #[default]
    Off,
    On,
    /// A dash: some of what the item stands for is on. No menu has such an item yet.
    #[allow(dead_code)]
    Mixed,
}

/// One choice in a menu.
#[derive(Clone)]
pub struct Item {
    pub title: SharedString,
    pub enabled: bool,
    pub check: Check,
    /// The key shown beside the title (the action's own key; the menu doesn't bind it).
    pub key: Option<KeyEquivalent>,
    pub icon: Option<Icon>,
    pub on_select: Option<Callback>,
}

impl Item {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), enabled: true, check: Check::Off, key: None, icon: None, on_select: None }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.check = if checked { Check::On } else { Check::Off };
        self
    }

    #[allow(dead_code)]
    pub fn check(mut self, check: Check) -> Self {
        self.check = check;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn key(mut self, key: Option<KeyEquivalent>) -> Self {
        self.key = key;
        self
    }

    pub fn icon(mut self, icon: Option<Icon>) -> Self {
        self.icon = icon;
        self
    }

    pub fn on_select(mut self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(run));
        self
    }
}

/// A row of a menu.
#[derive(Clone)]
pub enum Entry {
    Item(Item),
    Separator,
    /// A heading over what follows, not a choice.
    Header(SharedString),
    Submenu {
        title: SharedString,
        enabled: bool,
        menu: Menu,
    },
    /// A row of colour dots to choose one from, as Finder's tags in a file's context menu: one
    /// click picks and closes the menu.
    Swatches(Swatches),
}

/// A colour on offer in a [`Swatches`] row.
#[derive(Clone)]
pub struct Swatch {
    pub color: Hsla,
    /// The colour's name: its tooltip, and what VoiceOver says.
    pub name: SharedString,
}

impl Swatch {
    /// The dot that takes the colour away: an empty ring with a slash through it, not a colour (drawn so by its zero alpha).
    pub fn none(name: impl Into<SharedString>) -> Self {
        Self { color: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 }, name: name.into() }
    }
}

#[derive(Clone)]
pub struct Swatches {
    pub swatches: Vec<Swatch>,
    /// The one that is the current choice: ringed.
    pub selected: Option<usize>,
    pub on_pick: Pick,
}

/// A menu: its rows, and what to do when it closes.
#[derive(Clone, Default)]
pub struct Menu {
    pub entries: Vec<Entry>,
    /// Runs once the menu has closed, whether an item was chosen or not, and before the chosen
    /// item's own callback: for state that shows the menu is open (a row's focus ring).
    pub on_close: Option<Callback>,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn header(mut self, title: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Header(title.into()));
        self
    }

    pub fn item(mut self, item: Item) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// A separator, unless the menu is empty or ends in one already: a menu never starts or
    /// doubles one, whichever of the items around it were left out.
    pub fn separator(mut self) -> Self {
        if !matches!(self.entries.last(), None | Some(Entry::Separator)) {
            self.entries.push(Entry::Separator);
        }
        self
    }

    pub fn submenu(mut self, title: impl Into<SharedString>, menu: Menu) -> Self {
        self.entries.push(Entry::Submenu { title: title.into(), enabled: true, menu });
        self
    }

    /// A submenu that can't be opened (nothing to offer in it now), shown greyed out.
    pub fn disabled_submenu(mut self, title: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Submenu { title: title.into(), enabled: false, menu: Menu::new() });
        self
    }

    /// A row of colour dots (see [`Entry::Swatches`]); `selected` is the ringed one.
    pub fn swatches(
        mut self,
        swatches: Vec<Swatch>,
        selected: Option<usize>,
        on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(Entry::Swatches(Swatches { swatches, selected, on_pick: Rc::new(on_pick) }));
        self
    }

    pub fn when(self, condition: bool, build: impl FnOnce(Self) -> Self) -> Self {
        if condition { build(self) } else { self }
    }

    pub fn on_close(mut self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(run));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|entry| matches!(entry, Entry::Separator))
    }
}

// The text form is for devctl's `menu` and for tests.
#[cfg(any(test, feature = "devctl"))]
impl Menu {
    /// The rows as lines of text, indented by depth, for tests and devctl: `- title [x] [off]`
    /// for an item (`[x]` checked, `[-]` mixed, `[off]` disabled), `---` a separator, `# title` a
    /// header, `> title` a submenu, `~ Grey | Blue [x]` a row of colour dots (`[x]` the ringed one).
    pub fn outline(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.outline_into(0, &mut out);
        out
    }

    fn outline_into(&self, depth: usize, out: &mut Vec<String>) {
        let indent = "  ".repeat(depth);
        for entry in &self.entries {
            match entry {
                Entry::Separator => out.push(format!("{indent}---")),
                Entry::Header(title) => out.push(format!("{indent}# {title}")),
                Entry::Item(item) => out.push(format!("{indent}- {}", describe(item))),
                Entry::Swatches(row) => {
                    let names: Vec<String> = row
                        .swatches
                        .iter()
                        .enumerate()
                        .map(|(ix, swatch)| {
                            format!("{}{}", swatch.name, if row.selected == Some(ix) { " [x]" } else { "" })
                        })
                        .collect();
                    out.push(format!("{indent}~ {}", names.join(" | ")));
                }
                Entry::Submenu { title, enabled, menu } => {
                    out.push(format!("{indent}> {title}{}", if *enabled { "" } else { " [off]" }));
                    menu.outline_into(depth + 1, out);
                }
            }
        }
    }

    /// Every item that can be chosen, depth first, with the titles of the submenus it is in:
    /// what `menu-pick` numbers.
    pub fn leaves(&self) -> Vec<Leaf<'_>> {
        let mut out = Vec::new();
        self.leaves_into(&mut Vec::new(), &mut out);
        out
    }

    fn leaves_into<'a>(&'a self, path: &mut Vec<&'a str>, out: &mut Vec<Leaf<'a>>) {
        for entry in &self.entries {
            match entry {
                Entry::Item(item) => out.push(Leaf { path: path.join(" > "), item }),
                Entry::Submenu { title, menu, .. } => {
                    path.push(title);
                    menu.leaves_into(path, out);
                    path.pop();
                }
                Entry::Separator | Entry::Header(_) | Entry::Swatches(_) => {}
            }
        }
    }

    /// The first row of colour dots, which `menu-pick colour` chooses from.
    pub fn swatch_row(&self) -> Option<&Swatches> {
        self.entries.iter().find_map(|entry| match entry {
            Entry::Swatches(row) => Some(row),
            _ => None,
        })
    }
}

/// An item with the submenus above it.
#[cfg(any(test, feature = "devctl"))]
pub struct Leaf<'a> {
    /// Their titles, " > " apart; empty at the top level.
    pub path: String,
    pub item: &'a Item,
}

#[cfg(any(test, feature = "devctl"))]
impl Leaf<'_> {
    /// `Parent > Title`, or just the title.
    pub fn full_title(&self) -> String {
        if self.path.is_empty() {
            self.item.title.to_string()
        } else {
            format!("{} > {}", self.path, self.item.title)
        }
    }
}

#[cfg(any(test, feature = "devctl"))]
fn describe(item: &Item) -> String {
    let mut text = item.title.to_string();
    match item.check {
        Check::Off => {}
        Check::On => text.push_str(" [x]"),
        Check::Mixed => text.push_str(" [-]"),
    }
    if !item.enabled {
        text.push_str(" [off]");
    }
    text
}

/// A callback that updates `this` with the window and its context: the form most items take.
/// Nothing happens if the entity is gone by then.
pub fn run_on<T: 'static>(
    this: &WeakEntity<T>,
    run: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let this = this.clone();
    move |window, cx| {
        this.update(cx, |state, cx| run(state, window, cx)).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::{Check, Item, Menu, Swatch};

    fn menu() -> Menu {
        Menu::new()
            .header("Open in")
            .item(Item::new("Zed").checked(true))
            .separator()
            .separator()
            .submenu("More", Menu::new().item(Item::new("Cursor")).item(Item::new("Nova").enabled(false)))
            .disabled_submenu("None")
            .item(Item::new("Both").check(Check::Mixed))
    }

    #[test]
    fn the_outline_shows_every_row() {
        assert_eq!(
            menu().outline(),
            [
                "# Open in",
                "- Zed [x]",
                "---",
                "> More",
                "  - Cursor",
                "  - Nova [off]",
                "> None [off]",
                "- Both [-]"
            ]
        );
    }

    #[test]
    fn separators_never_double_or_lead() {
        let menu = Menu::new().separator().item(Item::new("A")).separator().separator();
        assert_eq!(menu.outline(), ["- A", "---"]);
        assert!(Menu::new().separator().is_empty());
    }

    #[test]
    fn a_row_of_swatches_shows_its_names_and_the_ringed_one() {
        let swatch =
            |name: &str| Swatch { color: gpui_kit::hsla(0.0, 0.0, 0.5, 1.0), name: name.to_string().into() };
        let menu = Menu::new()
            .item(Item::new("Rename…"))
            .separator()
            .swatches(vec![swatch("Grey"), swatch("Blue")], Some(1), |_, _, _| {})
            .separator()
            .item(Item::new("Ungroup"));
        assert_eq!(menu.outline(), ["- Rename…", "---", "~ Grey | Blue [x]", "---", "- Ungroup"]);
        let titles: Vec<String> = menu.leaves().iter().map(|leaf| leaf.full_title()).collect();
        assert_eq!(titles, ["Rename…", "Ungroup"], "the dots are not items to number");
        assert!(menu.swatch_row().is_some());
    }

    #[test]
    fn leaves_are_numbered_through_the_submenus() {
        let menu = menu();
        let titles: Vec<String> = menu.leaves().iter().map(|leaf| leaf.full_title()).collect();
        assert_eq!(titles, ["Zed", "More > Cursor", "More > Nova", "Both"]);
    }
}
