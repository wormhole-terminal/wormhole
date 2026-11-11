// The terminal's cell from the font's measurements, and how many cells a pane holds: the core's rule
// (`view::layout::cell`), which the GPUI app sizes its grid by too. `UniFFISource` installs it over the FFI; a build
// with no Rust (the renderer's own tests, the sample app) has `Plain`, the same arithmetic, which a real-FFI test
// (`CellRuleTests`) holds equal to the core's so the two cannot drift.

import Foundation
import Synchronization

/// One cell and where text sits in it, in points from the cell's top. The width is the font's `m` advance, unrounded.
public struct TerminalCell: Equatable, Sendable {
    public var width: Double
    public var height: Double
    public var baseline: Double
    public var underline: Double
    public var strike: Double

    public init(width: Double, height: Double, baseline: Double, underline: Double, strike: Double) {
        (self.width, self.height, self.baseline, self.underline, self.strike) = (width, height, baseline, underline, strike)
    }

    /// The thickness of underlines, strikethroughs and overlines, in points (the core's `LINE`).
    public static let line = 1.0
}

public protocol CellRules: Sendable {
    /// `advance`, `ascent` and `descent` (positive) are the font's at `fontSize`; `lineHeight` a multiple of the size.
    func cell(fontSize: Double, lineHeight: Double, advance: Double, ascent: Double, descent: Double) -> TerminalCell
    /// The whole cells an area of points holds (at least two columns and a row), `slack` short of a cell still counting.
    func fit(_ cell: TerminalCell, width: Double, height: Double, slack: Double) -> (cols: Int, rows: Int)
}

public enum CellRule {
    /// The core's arithmetic for a build without it (see the file's head).
    public struct Plain: CellRules {
        public init() {}
        public func cell(fontSize: Double, lineHeight: Double, advance: Double, ascent: Double, descent: Double) -> TerminalCell {
            let (size, a, d) = (Float(fontSize), Float(ascent), Float(descent))
            let height = (size * Float(lineHeight)).rounded()
            let baseline = (height - a - d) / 2 + a
            return TerminalCell(
                width: Double(Float(advance)), height: Double(height), baseline: Double(baseline),
                underline: Double((height - a + d) / 2 + a - d * 0.618), strike: Double((a * 0.5 + baseline) * 0.5))
        }

        public func fit(_ cell: TerminalCell, width: Double, height: Double, slack: Double) -> (cols: Int, rows: Int) {
            let cols = ((Float(width) + Float(slack)) / Float(cell.width)).rounded(.down)
            let rows = ((Float(height) + Float(slack)) / Float(cell.height)).rounded(.down)
            return (Int(max(2, cols)), Int(max(1, rows)))
        }
    }

    private static let installed = Mutex<any CellRules>(Plain())

    public static func install(_ rules: any CellRules) { installed.withLock { $0 = rules } }
    public static var current: any CellRules { installed.withLock { $0 } }
}
