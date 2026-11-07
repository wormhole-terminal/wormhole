// The row every list of a card shares: `state.hover` under the pointer, `state.selected` for the open file or
// commit, `state.active` while pressed, in the card-row corner (8). The selected fill is one shape that slides
// from row to row, which the GPUI app cuts; Reduce Motion cuts it too. The content is told whether the row is
// hovered (hover actions) and picks its own text colours for the selected tint.

import DesignSystem
import SwiftUI

extension EnvironmentValues {
    /// The namespace the selected fill slides in; nil where a list has no selection to slide.
    @Entry var selectionSpace: Namespace.ID?
}

struct PanelRow<Content: View>: View {
    var selected = false
    let action: () -> Void
    @ViewBuilder let content: (_ hovering: Bool) -> Content
    @State private var hovering = false

    var body: some View {
        Button(action: action) { content(hovering) }
            .buttonStyle(Surface(selected: selected, hovering: hovering))
            .onHover { hovering = $0 }
    }

    private struct Surface: ButtonStyle {
        let selected: Bool
        let hovering: Bool
        @Environment(\.tokens) private var tokens
        @Environment(\.selectionSpace) private var space
        @Environment(\.accessibilityReduceMotion) private var reduceMotion

        func makeBody(configuration: Configuration) -> some View {
            let shape = RoundedRectangle(cornerRadius: Radius.row, style: .continuous)
            configuration.label
                .background {
                    ZStack {
                        shape.fill(configuration.isPressed ? tokens.stateActive : hovering && !selected ? tokens.stateHover : .clear)
                        if selected {
                            if let space, !reduceMotion {
                                shape.fill(tokens.stateSelected).matchedGeometryEffect(id: "selection", in: space)
                            } else {
                                shape.fill(tokens.stateSelected)
                            }
                        }
                    }
                    .animation(reduceMotion ? nil : .snappy(duration: 0.15), value: hovering)
                }
                .contentShape(shape)
        }
    }
}
