// The container every list row shares: hover and selected fills in the row radius. Hover is the container's
// own business, and the content may ask for it (a hint that shows only under the pointer); selection is the owner's.

import SwiftUI

public struct Row<Content: View>: View {
    public let isSelected: Bool
    private let content: (Bool) -> Content
    @State private var hovering = false

    public init(isSelected: Bool = false, @ViewBuilder content: @escaping (_ hovering: Bool) -> Content) {
        self.isSelected = isSelected
        self.content = content
    }

    public var body: some View {
        content(hovering)
            .padding(.horizontal, Space._2)
            .background(fill, in: RoundedRectangle(cornerRadius: Radius.row))
            .contentShape(RoundedRectangle(cornerRadius: Radius.row))
            .onHover { hovering = $0 }
    }

    private var fill: Color {
        isSelected ? Palette.selected : hovering ? Palette.hover : .clear
    }
}
