// A window drawn as a sketch (`style::window_sketch`): the columns, the title row's parts, the status bar and the minimap
// as a few blocks, in the live theme's colours. Where each block goes is the core's (`SketchBlock`s); this paints them,
// snapped to device pixels so a one-point line stays one line.

import AuxModel
import DesignSystem
import SwiftUI

/// The window as the core lays it out, drawn small.
public struct WindowSketch: View {
    let blocks: [SketchBlock]
    @Environment(\.tokens) private var tokens
    @Environment(\.displayScale) private var scale

    /// `SKETCH_H`: tall enough for four lines of text and the bars around them.
    public static let height: CGFloat = 80
    /// How strong the chips and the minimap are drawn against the window (`CHIP_ALPHA`, `MINIMAP_ALPHA`).
    private static let softAlpha = 0.5

    /// A sketch of `blocks`.
    public init(blocks: [SketchBlock]) { self.blocks = blocks }

    /// The sketch.
    public var body: some View {
        Canvas { context, _ in
            for block in blocks {
                let rect = CGRect(x: snap(block.x), y: snap(block.y), width: snap(block.x + block.w) - snap(block.x), height: snap(block.y + block.h) - snap(block.y))
                let radius: CGFloat =
                    switch block.corner {
                    case .square: 0
                    case .soft: tokens.metrics.radiusSm
                    case .round: min(rect.width, rect.height) / 2
                    }
                context.fill(Path(roundedRect: rect, cornerRadius: radius), with: .color(color(block.fill)))
            }
        }
        .frame(height: Self.height)
        .frame(maxWidth: .infinity)
        // The sketch is the window's own colour on a card of another fill; Increase Contrast, where that may not be edge enough, outlines it.
        .background((tokens.rgba(named: "surface.window").map { RGBA($0.red, $0.green, $0.blue).color }) ?? tokens.surfaceWindow)
        .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        .overlay { if ThemeStore.shared.increaseContrast { RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).strokeBorder(tokens.border) } }
        .accessibilityHidden(true)
    }

    private func snap(_ points: Double) -> CGFloat { (points * scale).rounded() / scale }

    private func color(_ fill: SketchBlock.Fill) -> Color {
        switch fill {
        case .sidebar: tokens.sidebarBackground
        case .panelRight: tokens.surfacePanel
        case .rule: tokens.border
        case .selectedRow: tokens.sidebarRowSelected
        case .faint: tokens.textFaint
        case .muted: tokens.textMuted
        case .chip: tokens.primary.opacity(Self.softAlpha)
        case .primary: tokens.primary
        case .tab: tokens.stateSelected
        case .minimap: tokens.textFaint.opacity(Self.softAlpha)
        }
    }
}
