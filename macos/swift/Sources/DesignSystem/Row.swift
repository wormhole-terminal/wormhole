// The container every list row shares: hover, press and selected fills in the row radius. The selected fill is
// one shape that slides from row to row (`matchedGeometryEffect`) instead of switching off and on. Hover and press
// belong to the container, and the content may ask for hover (a hint that shows only under the pointer);
// selection is the owner's, and so is the namespace the selection shape slides in.

import SwiftUI

public struct Row<Content: View>: View {
    public let isSelected: Bool
    public let selection: Namespace.ID
    public let action: () -> Void
    private let content: (Bool) -> Content
    @State private var hovering = false
    @Environment(\.isEnabled) private var enabled

    public init(
        isSelected: Bool = false, selection: Namespace.ID, action: @escaping () -> Void,
        @ViewBuilder content: @escaping (_ hovering: Bool) -> Content
    ) {
        self.isSelected = isSelected
        self.selection = selection
        self.action = action
        self.content = content
    }

    public var body: some View {
        Button(action: action) { content(hovering).padding(.horizontal, Space._2) }
            .buttonStyle(RowStyle(isSelected: isSelected, hovering: hovering && enabled, selection: selection))
            .onHover { hovering = $0 }
            // A row with nothing to do now is drawn held back, with no hover and no press; VoiceOver hears the selection.
            .opacity(enabled ? 1 : 0.5)
            .accessibilityAddTraits(isSelected ? .isSelected : [])
            .motion(Motion.selection, value: isSelected)
    }
}

private struct RowStyle: ButtonStyle {
    @Environment(\.tokens) private var tokens
    let isSelected: Bool
    let hovering: Bool
    let selection: Namespace.ID

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: Radius.row)
        return configuration.label
            .background {
                ZStack {
                    shape.fill(configuration.isPressed ? tokens.stateActive : hovering ? tokens.stateHover : .clear)
                    if isSelected {
                        shape.fill(tokens.sidebarRowSelected).matchedGeometryEffect(id: "selection", in: selection)
                    }
                }
                .motion(Motion.press, value: hovering)
                .motion(Motion.press, value: configuration.isPressed)
            }
            .contentShape(shape)
            .pressed(configuration.isPressed)
    }
}
