// Moving one item of a list the person orders by hand (next steps, notes), as drag and drop and ⌥⌘↑ / ⌥⌘↓ do.
// Mirrors `wormhole_core::view::reorder`, whose rules the FFI will bring; the functions are this small.

public enum Reorder {
    /// Moves `items[from]` so it ends up at index `to` (an index after the move, so dropping onto an item puts the
    /// dragged one where that one was). Whether anything moved: past the end, or onto itself, nothing does.
    @discardableResult
    public static func move<T>(_ items: inout [T], from: Int, to: Int) -> Bool {
        guard from != to, items.indices.contains(from), items.indices.contains(to) else { return false }
        items.insert(items.remove(at: from), at: to)
        return true
    }

    /// Where item `index` of `count` goes one step up (`delta < 0`) or down; nil at either end, where a keyboard
    /// move stops instead of wrapping.
    public static func step(_ index: Int, _ delta: Int, count: Int) -> Int? {
        let target = index + delta
        return (0..<count).contains(index) && (0..<count).contains(target) && target != index ? target : nil
    }

    /// Which edge of the target a dragged item lands on: below when dragging down, above when dragging up.
    public enum Edge: Sendable { case above, below }

    public static func edge(from: Int, over target: Int) -> Edge? {
        from == target ? nil : (from < target ? .below : .above)
    }
}
