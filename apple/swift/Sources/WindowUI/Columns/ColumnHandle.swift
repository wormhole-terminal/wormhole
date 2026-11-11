// The draggable edge between two columns: a hairline (the theme's divider colour and width, drawn by the
// window along the column) with an 8 pt grip straddling it. It reports where the pointer is and the window
// resolves the width (`ColumnResize`), so one rule decides every drag; a double click puts the column back to
// its default, as native split views do; VoiceOver and the keyboard adjust it in steps.

import DesignSystem
import SwiftUI
import WordsModel

struct ColumnHandle: View {
    let label: String
    let width: CGFloat
    /// Called with the pointer's x in the window's space while dragging, and once more with nil at the end.
    let drag: @MainActor (CGFloat?) -> Void
    let reset: @MainActor () -> Void
    let step: @MainActor (CGFloat) -> Void
    private static let keyStep: CGFloat = 16

    var body: some View {
        Color.clear
            .frame(width: Chrome.resizeHandle)
            .contentShape(Rectangle())
            .pointerStyle(.columnResize)
            .gesture(
                DragGesture(minimumDistance: 1, coordinateSpace: .named(PaneSpace.name))
                    .onChanged { drag($0.location.x) }
                    .onEnded { _ in drag(nil) }
            )
            .onTapGesture(count: 2, perform: reset)
            .accessibilityElement()
            .accessibilityLabel(label)
            .accessibilityValue(Words.text("window-column-width", ["points": .whole(Int(width))], fallback: "\(Int(width)) points"))
            .accessibilityAdjustableAction { direction in
                switch direction {
                case .increment: step(Self.keyStep)
                case .decrement: step(-Self.keyStep)
                @unknown default: break
                }
            }
    }
}

/// The window's own coordinate space, which drags are measured in.
enum PaneSpace {
    static let name = "window-panes"
}

/// The words beside the pointer while a column is dragged: its width, and why it stopped ("430 · max") or what
/// letting go does ("Close").
struct ResizeReadout: View {
    let text: String
    let pointer: CGFloat
    @Environment(\.chrome) private var colours

    var body: some View {
        Text(text)
            .font(TextRole.caption.font.weight(.medium))
            .monospacedDigit()
            .foregroundStyle(colours.tooltipText)
            .padding(.horizontal, Space._2)
            .frame(height: Chrome.hit - Space._1)
            .background(colours.tooltip, in: RoundedRectangle(cornerRadius: colours.radius))
            .offset(x: pointer + Space._3, y: Chrome.titleHeight + Space._2)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
    }
}
