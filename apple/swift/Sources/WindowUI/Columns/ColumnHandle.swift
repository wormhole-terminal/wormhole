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
    /// Called with the pointer in the window's space while dragging, and once more with nil at the end.
    let drag: @MainActor (CGPoint?) -> Void
    let reset: @MainActor () -> Void
    let step: @MainActor (CGFloat) -> Void
    private static let keyStep: CGFloat = 16

    @State private var hot = false
    @State private var dragging = false
    @State private var hoverTask: Task<Void, Never>?
    @Environment(\.tokens) private var tokens
    /// How long the pointer rests on the handle before its line lights (`resize.rs`, so crossing it flashes nothing).
    private static let hoverDelay: Duration = .milliseconds(120)

    var body: some View {
        Rectangle().fill(.clear)
            // The line takes `resize.handle.hover` after a short rest and `resize.handle.active` while it is dragged.
            .overlay { if hot || dragging { Rectangle().fill(dragging ? tokens.resizeHandleActive : tokens.resizeHandleHover).frame(width: tokens.metrics.borderWidth) } }
            .frame(width: Chrome.resizeHandle)
            .contentShape(Rectangle())
            .pointerStyle(.columnResize)
            .onHover { inside in
                hoverTask?.cancel()
                guard inside else { return hot = false }
                hoverTask = Task {
                    try? await Task.sleep(for: Self.hoverDelay)
                    if !Task.isCancelled { hot = true }
                }
            }
            .gesture(
                DragGesture(minimumDistance: 1, coordinateSpace: .named(PaneSpace.name))
                    .onChanged {
                        dragging = true
                        drag($0.location)
                    }
                    .onEnded { _ in
                        dragging = false
                        drag(nil)
                    }
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
    let pointerY: CGFloat
    @Environment(\.chrome) private var colours

    var body: some View {
        Text(text)
            .font(TextRole.caption.font.weight(.medium))
            .monospacedDigit()
            .foregroundStyle(colours.tooltipText)
            .padding(.horizontal, Space._2)
            .frame(height: Chrome.hit - Space._1)
            .background(colours.tooltip, in: RoundedRectangle(cornerRadius: colours.radius))
            .offset(x: pointer + Space._2, y: pointerY - Space._2)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
    }
}
