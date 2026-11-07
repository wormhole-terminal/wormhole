// The filter field under the header: a faint rounded well with a magnifier, as System Settings draws its search.

import DesignSystem
import SwiftUI

struct SidebarSearchField: View {
    @Binding var text: String
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass").foregroundStyle(Palette.textSecondary).accessibilityHidden(true)
            TextField("Search", text: $text)
                .textFieldStyle(.plain)
                .focused($focused)
                .accessibilityLabel("Search terminals")
            if !text.isEmpty {
                Button { text = "" } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(Palette.textSecondary)
                }
                .buttonStyle(.plain)
                .motionTransition(.scale(scale: 0.5).combined(with: .opacity))
                .accessibilityLabel("Clear search")
            }
        }
        .textRole(.body)
        .padding(.horizontal, Space._2)
        .frame(height: Metric.control)
        .background(Palette.fill, in: RoundedRectangle(cornerRadius: Radius.row))
        .overlay {
            RoundedRectangle(cornerRadius: Radius.row).strokeBorder(Palette.focusRing, lineWidth: Metric.ring)
                .opacity(focused ? 1 : 0)
        }
        .motion(Motion.press, value: focused)
        .motion(Motion.selection, value: text.isEmpty)
        .padding(.horizontal, Metric.edgeInset)
    }
}
