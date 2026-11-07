// The draggable edge between two columns: a hairline with a wider invisible grip. It only edits a width
// the owner holds, inside its range; `grows` says which way a drag to the right moves the edge.

import DesignSystem
import SwiftUI

struct ColumnDivider: View {
    @Binding var width: CGFloat
    let range: ClosedRange<CGFloat>
    /// +1 when a drag to the right widens the column (sidebar), -1 when it narrows it (right panel).
    let grows: CGFloat
    let label: String
    /// A double click puts the column back to this, as native split views do.
    let initial: CGFloat
    @State private var widthAtStart: CGFloat?

    private static let grip: CGFloat = 8
    private static let keyStep: CGFloat = 16

    var body: some View {
        Palette.divider
            .frame(width: 1)
            .overlay {
                Color.clear
                    .frame(width: Self.grip)
                    .contentShape(Rectangle())
                    .pointerStyle(.columnResize)
                    .gesture(drag)
                    .onTapGesture(count: 2) { resize(to: initial) }
            }
            .accessibilityElement()
            .accessibilityLabel(label)
            .accessibilityValue("\(Int(width)) points")
            .accessibilityAdjustableAction { direction in
                switch direction {
                case .increment: resize(to: width + Self.keyStep)
                case .decrement: resize(to: width - Self.keyStep)
                @unknown default: break
                }
            }
    }

    private var drag: some Gesture {
        DragGesture(minimumDistance: 1, coordinateSpace: .global)
            .onChanged { value in
                let start = widthAtStart ?? width
                widthAtStart = start
                resize(to: start + grows * value.translation.width)
            }
            .onEnded { _ in widthAtStart = nil }
    }

    private func resize(to proposed: CGFloat) {
        width = min(max(proposed, range.lowerBound), range.upperBound)
    }
}
