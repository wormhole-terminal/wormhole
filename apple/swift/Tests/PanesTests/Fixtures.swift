// Trees for the tests, as `view/panes/tests.rs` builds them.

import PanesModel

let W = 800.0
let H = 600.0

func leaf(_ id: PaneID) -> Node { .leaf(id) }
func split(_ axis: Axis, _ children: [(Double, Node)]) -> Node { .split(axis, children.map { Child($0.0, $0.1) }) }

func tree(_ root: Node) -> PaneTree { PaneTree(repairing: root)! }

/// Ratios of the root's children, rounded to thousandths.
func shares(_ tree: PaneTree) -> [Double] {
    guard case .split(_, let children) = tree.root else { return [] }
    return children.map { ($0.ratio * 1000).rounded() / 1000 }
}

/// The 2×2: 1 and 2 on top, 3 and 4 below (two columns of two).
func grid() -> PaneTree {
    tree(split(.row, [(0.5, split(.column, [(0.5, leaf(1)), (0.5, leaf(3))])), (0.5, split(.column, [(0.5, leaf(2)), (0.5, leaf(4))]))]))
}

func divider(_ path: [Int], _ index: Int) -> DividerID { DividerID(path: path, index: index) }
