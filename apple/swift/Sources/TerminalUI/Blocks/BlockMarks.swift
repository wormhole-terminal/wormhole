// What the blocks draw beside and over the grid (`overlays::paint_blocks`): each block's gutter mark in the pane's
// left padding, from its first row to its last in view, and the hovered block's tint across its rows. A 2 pt bar
// 1 pt from the edge, inset where a block starts or ends in view so neighbours do not run into each other.
// The mark is the status colour for a command that finished, faint for one running or a prompt nothing ran at.

import DesignSystem
import SwiftUI

public struct BlockGeometry: Equatable, Sendable {
    public var rows: Int
    public var cellHeight: CGFloat
    /// The grid's top inside the view.
    public var top: CGFloat
    /// The terminal's left padding, where the marks stand.
    public var gutter: CGFloat

    public init(rows: Int, cellHeight: CGFloat, top: CGFloat = 0, gutter: CGFloat) {
        (self.rows, self.cellHeight, self.top, self.gutter) = (rows, cellHeight, top, gutter)
    }

    /// The viewport row at height `y` of the view, if it is inside the grid.
    public func row(atY y: CGFloat) -> Int? {
        let row = Int(((y - top) / max(cellHeight, 1)).rounded(.down))
        return (y >= top && row < rows) ? row : nil
    }
}

public struct BlockMarks: View {
    let spans: [BlockSpan]
    let hovered: UInt64?
    let geometry: BlockGeometry
    @Environment(\.tokens) private var tokens

    /// `MARK_W`, `MARK_X`, `MARK_INSET` in `terminal_view/blocks.rs`.
    private static let markWidth: CGFloat = 2
    private static let markX: CGFloat = 1
    private static let markInset: CGFloat = 2

    public init(spans: [BlockSpan], hovered: UInt64?, geometry: BlockGeometry) {
        (self.spans, self.hovered, self.geometry) = (spans, hovered, geometry)
    }

    public var body: some View {
        Canvas { context, size in
            for span in spans {
                let top = geometry.top + geometry.cellHeight * CGFloat(span.rows.lowerBound)
                let height = geometry.cellHeight * CGFloat(span.rows.count)
                if hovered == span.id {
                    context.fill(Path(CGRect(x: 0, y: top, width: size.width, height: height)), with: .color(tokens.stateHover))
                }
                guard let color = markColor(span.state) else { continue }
                let insetTop = span.starts ? Self.markInset : 0
                let insetBottom = span.rows.upperBound < geometry.rows ? Self.markInset : 0
                let mark = CGRect(x: Self.markX, y: top + insetTop, width: Self.markWidth, height: height - insetTop - insetBottom)
                context.fill(Path(roundedRect: mark, cornerRadius: Self.markWidth / 2), with: .color(color))
            }
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
        .motion(Motion.press, value: hovered)
    }

    func markColor(_ state: BlockState) -> Color? {
        switch state {
        case .prompt, .running: tokens.textFaint
        case .finished(code: 0?, _): tokens.statusSuccess
        case .finished(code: _?, _): tokens.statusDanger
        case .finished(code: nil, took: _?): tokens.textFaint
        case .finished(code: nil, took: nil): nil
        }
    }
}

extension View {
    /// Tracks the pointer over the pane for the hovered block; the pane under keeps its clicks.
    public func blockHover(_ controller: BlockController, geometry: BlockGeometry) -> some View {
        onContinuousHover { phase in
            switch phase {
            case let .active(point): controller.hover(row: geometry.row(atY: point.y))
            case .ended: controller.leave()
            }
        }
    }
}
