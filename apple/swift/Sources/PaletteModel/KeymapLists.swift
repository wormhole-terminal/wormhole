// What the palette's Actions and the shortcut sheet list comes from the core's keymap (`keymap/palette.rs` and
// `sheet.rs`): which actions a window can do, their words, order and keys, and the sheet's sections, rows, filter and
// columns. Behind this seam so the model knows nothing of Rust; `PlatformBridge` installs the core's. A build without
// the FFI lists nothing (previews and tests install their own).

import Foundation
import Synchronization

/// What the window says about itself for the action list: the menus' facts (`MenuState`) that decide what is offered.
public struct ActionFacts: Sendable, Equatable {
    public var off: [String]
    public var disabled: [String]
    public var hidden: [String]
    public var isSplit: Bool
    public var isDebug: Bool
    public var isPersistent: Bool

    public init(
        off: [String] = [], disabled: [String] = [], hidden: [String] = [], isSplit: Bool = false, isDebug: Bool = false,
        isPersistent: Bool = false
    ) {
        (self.off, self.disabled, self.hidden) = (off, disabled, hidden)
        (self.isSplit, self.isDebug, self.isPersistent) = (isSplit, isDebug, isPersistent)
    }
}

/// One action of the keymap as the palette lists it.
public struct ActionRow: Sendable, Equatable {
    public var id: String
    public var label: String
    public var key: Shortcut?
    public var section: ActionSection

    public init(id: String, label: String, key: Shortcut? = nil, section: ActionSection) {
        (self.id, self.label, self.key, self.section) = (id, label, key, section)
    }
}

/// The actions a window offers, most used first, and the ids suggested with nothing typed.
public struct ActionList: Sendable, Equatable {
    public var rows: [ActionRow]
    public var suggested: [String]

    public init(rows: [ActionRow] = [], suggested: [String] = []) { (self.rows, self.suggested) = (rows, suggested) }
}

public protocol KeymapLists: Sendable {
    func actions(_ facts: ActionFacts) -> ActionList
    /// The sheet without the rows of `hidden` actions, filtered by `query`.
    func sheet(hidden: [String], query: String) -> FilteredShortcuts
    /// How sections of these heights go into at most `count` columns.
    func columns(_ heights: [Int], _ count: Int) -> [Range<Int>]
}

/// The lists the palette and the sheet read: core's once installed, nothing before.
public enum ListedKeymap {
    private static let installed = Mutex<any KeymapLists>(NoLists())

    /// Makes `lists` the app's. Call once, at launch.
    public static func install(_ lists: any KeymapLists) { installed.withLock { $0 = lists } }

    public static func actions(_ facts: ActionFacts) -> ActionList { installed.withLock { $0 }.actions(facts) }

    public static func sheet(hidden: [String], query: String) -> FilteredShortcuts {
        installed.withLock { $0 }.sheet(hidden: hidden, query: query)
    }

    public static func columns(_ heights: [Int], _ count: Int) -> [Range<Int>] { installed.withLock { $0 }.columns(heights, count) }

    private struct NoLists: KeymapLists {
        func actions(_ facts: ActionFacts) -> ActionList { ActionList() }
        func sheet(hidden: [String], query: String) -> FilteredShortcuts { FilteredShortcuts() }
        func columns(_ heights: [Int], _ count: Int) -> [Range<Int>] { heights.isEmpty ? [] : [0..<heights.count] }
    }
}
