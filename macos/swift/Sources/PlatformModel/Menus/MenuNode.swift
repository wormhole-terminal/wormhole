// A menu, whoever draws it: titles, checks, keys, pictures, separators, headers, submenus and what choosing an
// item does, as plain values (`keymap::menus::Item` and the app's `platform::Menu` in one). PlatformUI turns it
// into an `NSMenu`; tests read it as text.

import Foundation

/// What choosing an item does: one of the keymap's actions, a pick from a dynamic part, or a context menu's own.
public enum MenuCommand: Hashable, Sendable {
    case action(ActionID)
    case openRecent(Int)
    case runProjectAction(String)
    case newTerminalWith(Int)
    case switchProfile(Int)
    case openInProfile(Int)
    case moveToProfile(Int)
    case context(ContextAction)
    /// From the status item, the Dock menu and the quick panel.
    case ambient(AmbientCommand)
}

/// A context menu's choice, named by its menu: `kind` says what, `subject` which row or group, `argument` the pick.
public struct ContextAction: Hashable, Sendable {
    public var kind: String
    public var subject: Int64?
    public var argument: String?

    public init(_ kind: String, subject: Int64? = nil, argument: String? = nil) {
        self.kind = kind
        self.subject = subject
        self.argument = argument
    }
}

/// A small picture before an item's title.
public enum MenuIcon: Hashable, Sendable {
    /// An SF Symbol by name, as Apple's own menus carry (`doc.on.doc` beside Copy).
    case symbol(String)
    /// A file's or folder's icon as Finder draws it.
    case file(path: String)
    /// The generic folder, for one on a share that must not be touched.
    case folder
    /// What a terminal is doing, in the status colours the status item's menu uses.
    case status(StatusGlyph)
}

public struct MenuItem: Hashable, Sendable {
    public var command: MenuCommand
    public var title: String
    public var key: KeySpec?
    public var checked = false
    public var enabled = true
    public var icon: MenuIcon?
    /// Right-aligned on the title's line: how long a terminal has waited ("2m"), or a count.
    public var trailing: String?
    /// A second line under the title (macOS 14.4+), also the tooltip.
    public var subtitle: String?

    public init(
        _ command: MenuCommand, _ title: String, key: KeySpec? = nil, checked: Bool = false, enabled: Bool = true,
        icon: MenuIcon? = nil, trailing: String? = nil, subtitle: String? = nil
    ) {
        self.command = command
        self.title = title
        self.key = key
        self.checked = checked
        self.enabled = enabled
        self.icon = icon
        self.trailing = trailing
        self.subtitle = subtitle
    }
}

/// A row of colour dots to pick from, as Finder's tags in a context menu; a click picks and closes.
public struct SwatchRow: Hashable, Sendable {
    public struct Swatch: Hashable, Sendable {
        public var name: String
        /// The theme token the dot is drawn from; nil is the dot that takes the colour away.
        public var token: String?
        public init(name: String, token: String?) { (self.name, self.token) = (name, token) }
    }

    public var swatches: [Swatch]
    public var selected: Int?
    public var command: @Sendable (Int) -> ContextAction

    public init(swatches: [Swatch], selected: Int?, command: @escaping @Sendable (Int) -> ContextAction) {
        (self.swatches, self.selected, self.command) = (swatches, selected, command)
    }

    public static func == (a: SwatchRow, b: SwatchRow) -> Bool { a.swatches == b.swatches && a.selected == b.selected }
    public func hash(into hasher: inout Hasher) { hasher.combine(swatches); hasher.combine(selected) }
}

public indirect enum MenuNode: Hashable, Sendable {
    case item(MenuItem)
    case separator
    /// A heading over what follows, not a choice.
    case header(String)
    /// The system's Services submenu.
    case services
    /// `trailing`: a count shown right-aligned on the title's line ("Other Terminals  3").
    case submenu(title: String, items: [MenuNode], enabled: Bool = true, trailing: String? = nil)
    /// A greyed line saying why a submenu has nothing in it ("No Recent Folders"), as Finder's do.
    case placeholder(String)
    case swatches(SwatchRow)
}

public struct Menu: Hashable, Sendable {
    public var title: String
    public var items: [MenuNode]
    public init(title: String, items: [MenuNode]) { (self.title, self.items) = (title, items) }
}
