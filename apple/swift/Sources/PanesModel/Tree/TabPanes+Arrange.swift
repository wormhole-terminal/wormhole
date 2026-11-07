// What a drop, a divider drag and a preset do to a tab's panes (`TabPanes`'s other half): each keeps the
// minimum sizes and the focus rules, and says why when it refuses.

import Foundation

extension TabPanes {
    /// A terminal dragged in from the sidebar onto `target`'s `zone`.
    public mutating func join(_ id: PaneID, beside target: PaneID, zone: Zone, room: PaneRoom) throws(DropError) {
        var next = tree
        guard next.insertBeside(target, zone: zone, new: id) else { throw .unknown }
        guard fits(next, [target, id], room) else { throw .tooSmall }
        (tree, focused, zoomed) = (next, id, nil)
    }

    /// A pane's header dragged onto another pane's edge.
    public mutating func move(_ id: PaneID, beside target: PaneID, zone: Zone, room: PaneRoom) throws(DropError) {
        guard tree.contains(id), tree.contains(target) else { throw .unknown }
        var next = tree
        guard next.move(id, beside: target, zone: zone) else { throw .unchanged }
        guard fits(next, [target, id], room) else { throw .tooSmall }
        (tree, focused, zoomed) = (next, id, nil)
    }

    /// Why a drop would be refused, nil when it would work: for the hint's colour, without doing it.
    public func joinRefusal(_ id: PaneID, beside target: PaneID, zone: Zone, room: PaneRoom) -> DropError? {
        var copy = self
        do { try copy.join(id, beside: target, zone: zone, room: room) } catch { return error }
        return nil
    }

    public func moveRefusal(_ id: PaneID, beside target: PaneID, zone: Zone, room: PaneRoom) -> DropError? {
        var copy = self
        do { try copy.move(id, beside: target, zone: zone, room: room) } catch { return error }
        return nil
    }

    /// A pane's terminal dragged to the sidebar: it leaves the split.
    public mutating func leave(_ id: PaneID) -> Leave? {
        guard close(id) else { return nil }
        return isSplit ? .shrunk : .dissolved
    }

    public mutating func drag(_ divider: DividerID, to position: Double, room: PaneRoom) -> Double? {
        tree.drag(divider, to: position, width: room.width, height: room.height, minWidth: room.minWidth, minHeight: room.minHeight)
    }

    public mutating func equalize(_ divider: DividerID) -> Bool { tree.equalize(divider) }

    /// The pane's share of its split as a percentage, set out of its neighbour's (the pane menu's).
    public mutating func setShare(of pane: PaneID, beside: PaneID, percent: Double, room: PaneRoom) -> Double? {
        tree.setShare(
            of: pane, beside: beside, to: percent / 100, width: room.width, height: room.height,
            minWidth: room.minWidth, minHeight: room.minHeight
        ).map { $0 * 100 }
    }

    public func neighbour(_ direction: Direction, room: PaneRoom) -> PaneID? {
        tree.neighbour(of: focused, direction, width: room.width, height: room.height)
    }

    public func plan(_ preset: Preset) -> PresetPlan { tree.plan(preset, focused: focused) }

    /// Arranges the tab as `preset`; `newIDs` fill what the panes don't. Returns the panes that leave, nil
    /// when `newIDs` is short.
    public mutating func arrange(_ preset: Preset, newIDs: [PaneID]) -> [PaneID]? {
        let plan = plan(preset)
        guard newIDs.count >= plan.new, tree.arrange(preset, plan.keep + newIDs.prefix(plan.new)) else { return nil }
        zoomed = nil
        settle()
        return plan.out
    }
}
