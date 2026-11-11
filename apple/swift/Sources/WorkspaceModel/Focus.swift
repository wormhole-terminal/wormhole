// Keyboard focus as the core decides it (`wormhole_core::view::focus`): F6's order of areas, ⇥'s stops, the Esc
// ladder and how a list is walked. The window holds the real focus and the facts; it hands them to these rules and
// acts on the answer. `WorkspaceBridge` installs the core's rules; without them (no FFI) every question answers
// "the terminal" and a list does not move, so a build without the core is inert, never different.

import Foundation
import Synchronization

/// One of a window's areas, in focus order.
public enum FocusArea: String, CaseIterable, Sendable {
    case titleRow = "title-row", sidebar, tabs, terminal, panelTabs = "panel-tabs", panelContent = "panel-content"
    case statusBar = "status-bar"
}

/// One ⇥ stop: an area and which of its stops (the title row's buttons, the panel's lists).
public struct FocusStop: Equatable, Sendable {
    public var area: FocusArea
    public var index: Int
    public init(_ area: FocusArea, _ index: Int = 0) { (self.area, self.index) = (area, index) }
}

/// What Esc does, the nearest open thing first.
public enum EscapeRung: Equatable, Sendable { case toShell, closePopup, closeDetail, toTerminal }

/// One item of a list as the keyboard walks it, and what a key does with the focus on one.
public struct ListItem: Equatable, Sendable {
    public var level: Int
    /// `true` for an open band, `false` for a folded one, `nil` for an item that does not fold.
    public var isOpen: Bool?
    public init(level: Int, isOpen: Bool?) { (self.level, self.isOpen) = (level, isOpen) }
}

public enum ListKey: Sendable { case up, down, left, right, first, last }

public enum ListMove: Equatable, Sendable { case to(Int), open(Int), fold(Int), stay }

public protocol FocusRules: Sendable {
    /// F6 (`delta` 1) or ⇧F6 (-1): the next area among those `shown`.
    func step(from: FocusArea?, delta: Int, shown: Set<FocusArea>) -> FocusArea
    /// ⇥ or ⇧⇥: the next stop; `stops` is how many each area has now (0 when hidden or out of Tab's reach).
    func tabStep(from: FocusStop?, delta: Int, stops: [FocusArea: Int]) -> FocusStop
    func escape(terminal: Bool, popup: Bool, detail: Bool) -> EscapeRung
    func walk(_ items: [ListItem], from: Int?, _ key: ListKey) -> ListMove
    /// The item `typed` jumps to, looking from `from`.
    func typeAhead(_ names: [String], from: Int?, _ typed: String) -> Int?
    /// Whether this press folds or opens a band: Space does, ↩ does not.
    func pressFoldsBand(space: Bool) -> Bool
}

public enum Focus {
    private static let installed = Mutex<(any FocusRules)?>(nil)

    public static func install(_ rules: (any FocusRules)?) { installed.withLock { $0 = rules } }

    public static var rules: any FocusRules { installed.withLock { $0 } ?? Inert() }

    private struct Inert: FocusRules {
        func step(from: FocusArea?, delta: Int, shown: Set<FocusArea>) -> FocusArea { .terminal }
        func tabStep(from: FocusStop?, delta: Int, stops: [FocusArea: Int]) -> FocusStop { FocusStop(.terminal) }
        func escape(terminal: Bool, popup: Bool, detail: Bool) -> EscapeRung { terminal ? .toShell : .toTerminal }
        func walk(_ items: [ListItem], from: Int?, _ key: ListKey) -> ListMove { .stay }
        func typeAhead(_ names: [String], from: Int?, _ typed: String) -> Int? { nil }
        func pressFoldsBand(space: Bool) -> Bool { space }
    }
}
