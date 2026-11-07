// A split's panes as the window keeps them: the tree, the pane that has the keyboard and the one zoomed to
// fill the area. Every change of the tree goes through here so the focus never names a pane that is gone.

import Foundation

/// The area the panes fill and what a pane must keep (0 × 0: not laid out yet, nothing is refused).
public struct PaneRoom: Sendable, Equatable {
    public var width, height, minWidth, minHeight: Double

    public init(_ width: Double, _ height: Double, minWidth: Double = PaneMin.width, minHeight: Double = PaneMin.height) {
        (self.width, self.height, self.minWidth, self.minHeight) = (width, height, minWidth, minHeight)
    }
}

public enum DropError: Error, Sendable, Equatable {
    /// A pane that isn't here.
    case unknown
    /// A pane would end up under its minimum size.
    case tooSmall
    /// The panes would end up arranged as they are.
    case unchanged
}

public enum Leave: Sendable, Equatable {
    /// The split keeps at least two panes.
    case shrunk
    /// One pane is left: the split is none.
    case dissolved
}

public struct TabPanes: Sendable, Equatable {
    public internal(set) var tree: PaneTree
    public internal(set) var focused: PaneID
    public internal(set) var zoomed: PaneID?

    public static func single(_ id: PaneID) -> TabPanes { TabPanes(tree: .single(id), focused: id, zoomed: nil) }

    /// The panes of a tab holding the terminals `ids`: a saved layout kept as far as it still names them
    /// (its focus and zoom with it), the rest appended at the right.
    public static func restored(_ saved: TabPanes?, ids: [PaneID]) -> TabPanes? {
        guard let first = ids.first else { return nil }
        var panes = saved.flatMap { $0.panes.contains(where: ids.contains) ? $0 : nil } ?? .single(first)
        for id in panes.tree.panes where !ids.contains(id) { panes.tree.remove(id) }
        for id in ids { panes.tree.append(id) }
        panes.settle()
        return panes
    }

    public var panes: [PaneID] { tree.panes }
    public var isSplit: Bool { tree.count > 1 }

    /// Gives the keyboard to `id`; a zoom on another pane ends.
    @discardableResult
    public mutating func focus(_ id: PaneID) -> Bool {
        guard tree.contains(id) else { return false }
        focused = id
        if let zoomed, zoomed != id { self.zoomed = nil }
        return true
    }

    /// ⌘D / ⇧⌘D: splits the focused pane; the new one takes the keyboard.
    @discardableResult
    public mutating func split(_ axis: Axis, new id: PaneID) -> Bool {
        guard tree.split(focused, axis: axis, new: id) else { return false }
        (focused, zoomed) = (id, nil)
        return true
    }

    /// Closes a pane; if it had the keyboard that goes to the next pane in reading order, else the previous.
    @discardableResult
    public mutating func close(_ id: PaneID) -> Bool {
        let order = tree.panes
        let at = order.firstIndex(of: id)
        guard tree.remove(id) else { return false }
        if focused == id, let at { focused = order[safe: at + 1] ?? order[safe: at - 1] ?? id }
        if zoomed == id { zoomed = nil }
        settle()
        return true
    }

    /// ⇧⌘↩: zooms the focused pane over the others, or restores them. Nothing in a single pane.
    @discardableResult
    public mutating func toggleZoom() -> Bool {
        guard isSplit else { return false }
        zoomed = zoomed == focused ? nil : focused
        return true
    }

    /// The panes on screen: a zoomed one alone fills the area.
    public func shown(width: Double, height: Double) -> [(id: PaneID, rect: PaneRect)] {
        if let zoomed { return [(zoomed, PaneRect(0, 0, width, height))] }
        return tree.rects(width: width, height: height)
    }

    public func dividers(width: Double, height: Double) -> [Divider] {
        zoomed == nil ? tree.dividers(width: width, height: height) : []
    }

    mutating func settle() {
        if !tree.contains(focused) { focused = tree.panes[0] }
        if let zoomed, !tree.contains(zoomed) { self.zoomed = nil }
        if !isSplit { zoomed = nil }
    }

    func fits(_ tree: PaneTree, _ ids: [PaneID], _ room: PaneRoom) -> Bool {
        guard room.width > 0, room.height > 0 else { return true }
        return tree.rects(width: room.width, height: room.height).filter { ids.contains($0.id) }
            .allSatisfy { $0.rect.w + 0.01 >= room.minWidth && $0.rect.h + 0.01 >= room.minHeight }
    }

    init(tree: PaneTree, focused: PaneID, zoomed: PaneID?) { (self.tree, self.focused, self.zoomed) = (tree, focused, zoomed) }
}
