// The core's cell (`wormhole_core::view::layout::cell`) behind `CellRule`: the renderer's grid sized by the GPUI app's rule.

import TerminalCore
import WormholeBindings

private struct CoreCellRules: CellRules {
    func cell(fontSize: Double, lineHeight: Double, advance: Double, ascent: Double, descent: Double) -> TerminalCell {
        let cell = terminalCell(fontSize: fontSize, lineHeight: lineHeight, advance: advance, ascent: ascent, descent: descent)
        return TerminalCell(width: cell.width, height: cell.height, baseline: cell.baseline, underline: cell.underline, strike: cell.strike)
    }

    func fit(_ cell: TerminalCell, width: Double, height: Double, slack: Double) -> (cols: Int, rows: Int) {
        let record = CellRecord(width: cell.width, height: cell.height, baseline: cell.baseline, underline: cell.underline, strike: cell.strike)
        let fit = terminalGridFit(cell: record, width: width, height: height, slack: slack)
        return (Int(fit.cols), Int(fit.rows))
    }
}

/// Makes the core's cell the one every terminal is sized by.
public func installCellRules() { CellRule.install(CoreCellRules()) }

private struct CoreCellShapes: CellShapeSource {
    func shapes(_ scalar: UInt32, cellWidth: Float, height: Float, phase: Float, fontSize: Float, scale: Float) -> (shapes: [CellShape], alpha: Float)? {
        guard let found = cellShapes(scalar: scalar, cellWidth: cellWidth, height: height, phase: phase, fontSize: fontSize, scale: scale)
        else { return nil }
        return (found.shapes.map { CellShape(rect: $0.rect, points: $0.points, width: $0.width) }, found.alpha)
    }
}

/// Makes the core's box drawing and block elements the renderer's.
public func installCellShapes() { CellShapes.install(CoreCellShapes()) }
