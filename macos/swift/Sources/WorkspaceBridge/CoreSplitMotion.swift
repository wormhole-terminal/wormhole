// A split's opening, asked of the core (`splitOpening`, `splitOpeningFrame`: `view::motion::split`): Swift hands over the
// rectangles it has and draws the rooms that come back.

import PanesModel
import WorkspaceModel
import WormholeBindings

public struct CoreSplitMotion: SplitMotion {
    public init() {}

    public func opening(before: [PanePlace], after: [PanePlace], new: PaneID) -> SplitOpening? {
        guard
            let found = splitOpening(before: before.map(SplitPlace.init), after: after.map(SplitPlace.init), new: Int64(new))
        else { return nil }
        return SplitOpening(
            target: Int(found.target), row: found.row, whole: WorkspaceModel.PaneRect(found.whole), newPane: WorkspaceModel.PaneRect(found.newPane),
            duration: splitOpeningDurationMs() / 1000)
    }

    public func frame(of opening: SplitOpening, elapsed: Double) -> SplitFrame {
        let record = SplitOpeningRecord(
            target: Int64(opening.target), row: opening.row, whole: SplitRect(opening.whole), newPane: SplitRect(opening.newPane))
        let at = splitOpeningFrame(openingRecord: record, elapsedMs: elapsed * 1000)
        return SplitFrame(kept: WorkspaceModel.PaneRect(at.kept), newPane: WorkspaceModel.PaneRect(at.newPane))
    }
}

extension SplitRect {
    init(_ rect: WorkspaceModel.PaneRect) { self.init(x: Float(rect.x), y: Float(rect.y), w: Float(rect.w), h: Float(rect.h)) }
}

extension SplitPlace {
    init(_ place: PanePlace) { self.init(id: Int64(place.id), rect: SplitRect(place.rect)) }
}

extension WorkspaceModel.PaneRect {
    init(_ rect: SplitRect) { self.init(Double(rect.x), Double(rect.y), Double(rect.w), Double(rect.h)) }
}
