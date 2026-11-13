// How the keyboard walks a tab of the panel (`window/focus/panel.rs`): the tab lists the rows the keys can be on, the
// core's `focus` rules say where each key goes (never a rule of Swift's), and the tab says what a press, a fold and a copy do
// on a row. One walk for Changes, History, Notes and Files.

import Foundation
import WorkspaceModel

/// One row the keys can be on. A band is a section's head: a ⇥ stop, folded only by Space.
public struct WalkRow: Equatable, Sendable {
    public var id: String
    public var level: Int
    /// `true` open, `false` folded, `nil` a row that does not fold.
    public var isOpen: Bool?
    public var isBand: Bool
    /// What type-ahead matches.
    public var name: String

    public init(id: String, level: Int, isOpen: Bool? = nil, isBand: Bool = false, name: String = "") {
        (self.id, self.level, self.isOpen, self.isBand, self.name) = (id, level, isOpen, isBand, name)
    }
}

@MainActor
public protocol PanelWalking: AnyObject {
    var walkRows: [WalkRow] { get }
    /// The row the keys are on.
    var cursor: String? { get set }
    /// A diff or viewer is open: it follows the keys from file to file.
    var following: Bool { get }
    /// ↩ or Space on a row that is not a band. `following`: the keys only moved onto it with a diff open.
    func press(_ row: WalkRow, space: Bool, following: Bool)
    /// → or ← on a row that folds.
    func fold(_ row: WalkRow)
    /// What ⌘C puts on the clipboard for the row; nil when the tab copies nothing there.
    func copy(_ row: WalkRow) -> String?
    /// What the notice chip says of that copy ("Copied commit hash"); nil: no chip.
    var copiedAs: PanelCopied? { get }
    /// A row the keys moved over while ⇧ was held (a range being marked).
    func mark(_ row: WalkRow)
}

extension PanelWalking {
    public var following: Bool { false }
    public func copy(_ row: WalkRow) -> String? { nil }
    public var copiedAs: PanelCopied? { nil }
    public func mark(_ row: WalkRow) {}

    /// A key that moves the keys' place; ⇧ marks the rows passed over.
    public func walk(_ key: ListKey, marking: Bool = false) {
        let rows = walkRows
        let from = place(in: rows)
        switch Focus.rules.walk(rows.map { ListItem(level: $0.level, isOpen: $0.isOpen) }, from: from, key) {
        case let .to(index): move(to: rows[index], marking: marking, from: from.map { rows[$0] })
        case let .open(index), let .fold(index): fold(rows[index])
        case .stay: break
        }
    }

    /// ↩ and Space: a band folds only on Space, anything else is pressed.
    public func pressCursor(space: Bool) {
        let rows = walkRows
        guard let at = place(in: rows) else { return }
        if rows[at].isBand {
            if Focus.rules.pressFoldsBand(space: space) { fold(rows[at]) }
        } else {
            press(rows[at], space: space, following: false)
        }
    }

    /// The row type-ahead jumps to for `typed`.
    public func typeAhead(_ typed: String) {
        let rows = walkRows
        guard let index = Focus.rules.typeAhead(rows.map(\.name), from: place(in: rows), typed) else { return }
        move(to: rows[index], marking: false, from: nil)
    }

    public func copyCursor() -> String? {
        let rows = walkRows
        return place(in: rows).flatMap { copy(rows[$0]) }
    }

    /// The rows each ⇥ stop starts at (`panel_lists`): every band, and the first row when it is not one.
    public var listStarts: [Int] {
        let rows = walkRows
        var starts = rows.indices.filter { rows[$0].isBand }
        if !rows.isEmpty, starts.first != 0 { starts.insert(0, at: 0) }
        return starts
    }

    /// The ⇥ stop the cursor is in (the first one while it is nowhere).
    public var listAt: Int {
        let rows = walkRows
        guard let at = place(in: rows) else { return 0 }
        return listStarts.lastIndex { $0 <= at } ?? 0
    }

    /// ⇥ into the `index`th stop (past the last: the last): the cursor on its first row, unless it is already in that one.
    public func enter(list index: Int) {
        let rows = walkRows
        let starts = listStarts
        guard !starts.isEmpty else { return }
        let stop = min(max(index, 0), starts.count - 1)
        if place(in: rows) != nil, listAt == stop { return }
        cursor = rows[starts[stop]].id
    }

    private func place(in rows: [WalkRow]) -> Int? { cursor.flatMap { id in rows.firstIndex { $0.id == id } } }

    private func move(to row: WalkRow, marking: Bool, from: WalkRow?) {
        if marking, let from { mark(from) }
        cursor = row.id
        if marking { mark(row) }
        if following, !row.isBand { press(row, space: false, following: true) }
    }
}

/// The letters typed into a list, kept while they come within a second of each other (`focus::list::TYPE_AHEAD_WINDOW`).
public struct TypedAhead {
    private var typed = ""
    private var last = Date.distantPast

    public init() {}

    public mutating func push(_ text: String, now: Date = Date()) -> String {
        if now.timeIntervalSince(last) > 1 { typed = "" }
        typed += text
        last = now
        return typed
    }
}
