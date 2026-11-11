// How much of the title row fits is the core's rule (`view::layout::title_row::fit_row`, through the FFI): the path loses its
// middle, then the status words go, then the extras one at a time, the branch chip shrinks to its glyph, Open in… and the
// splits fold into one `⋯`, the name is cut, and last the path folds to its icon. This module measures nothing and decides
// nothing of it: it hands the widths over (`RowWants`) and draws what comes back (`RowFit`).

import Foundation

public protocol RowFitting: Sendable {
    /// What the row draws in `avail` points.
    func fit(avail: Double, _ wants: RowWants) -> RowFit
}

/// The sample's fit, with no core to ask: every part at its natural width, nothing gives way.
public struct NaturalFit: RowFitting {
    public init() {}

    public func fit(avail: Double, _ wants: RowWants) -> RowFit {
        RowFit(
            path: wants.path.isEmpty ? .none : .step(0), words: wants.words > 0, chip: wants.chip?.natural, extras: wants.extras,
            portsStep: 0, folded: false, name: wants.name)
    }
}
