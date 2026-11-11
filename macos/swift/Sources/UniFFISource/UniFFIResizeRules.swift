// The core's live-resize rules (`wormhole_core::view::motion::live_resize`) behind `ResizeSteps`: one FFI call per question.

import CoreGraphics
import Foundation
import TerminalCore
import WormholeBindings

private func record(_ size: CGSize) -> SizeRecord { SizeRecord(width: Double(size.width), height: Double(size.height)) }
private func size(_ record: SizeRecord) -> CGSize { CGSize(width: record.width, height: record.height) }
private func record(_ frame: CGRect) -> WindowFrameRecord {
    WindowFrameRecord(x: Double(frame.minX), y: Double(frame.minY), width: Double(frame.width), height: Double(frame.height))
}

private final class Gate: ResizeGating {
    private let gate = LiveResizeGate()

    func step(live: Bool, differs: Bool) -> ResizeStep {
        let step = gate.step(live: live, differs: differs)
        return ResizeStep(apply: step.apply, wakeIn: step.wakeInMs.map { TimeInterval($0) / 1000 })
    }
}

private struct CoreResizeRules: ResizeRules {
    func snapped(_ proposed: CGSize, from: CGSize, spare: CGSize, cell: CGSize, least: CGSize) -> CGSize {
        size(resizeSnapped(proposed: record(proposed), from: record(from), spare: record(spare), cell: record(cell), least: record(least)))
    }

    func settle(spare: CGSize, cell: CGSize) -> CGSize? { resizeSettle(spare: record(spare), cell: record(cell)).map(size) }

    func settledFrame(_ frame: CGRect, strip: CGSize, visible: CGRect, heldLeft: Bool, heldTop: Bool) -> CGRect? {
        resizeSettledFrame(window: record(frame), strip: record(strip), visible: record(visible), heldLeft: heldLeft, heldTop: heldTop)
            .map { CGRect(x: $0.x, y: $0.y, width: $0.width, height: $0.height) }
    }

    func makeGate() -> any ResizeGating { Gate() }
}

/// Makes the core the live-resize rules of every window and terminal.
public func installResizeRules() { ResizeSteps.install(CoreResizeRules()) }
