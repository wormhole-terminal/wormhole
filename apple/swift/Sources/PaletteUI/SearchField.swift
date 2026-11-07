// The overlays' search box: a magnifier, a borderless field and an optional scope chip.

import DesignSystem
import SwiftUI

struct SearchField: View {
    let prompt: String
    @Binding var text: String
    var scope: String?
    var focus: FocusState<Bool>.Binding

    var body: some View {
        HStack(spacing: Space._3) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 16, weight: .regular))
                .foregroundStyle(Palette.textSecondary)
                .accessibilityHidden(true)
            if let scope {
                Text(scope)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(Color.accentColor)
                    .padding(.horizontal, Space._2)
                    .padding(.vertical, Space._1)
                    .background(Capsule().fill(Palette.selected))
                    .transition(.scale(scale: 0.9).combined(with: .opacity))
            }
            TextField(prompt, text: $text)
                .textFieldStyle(.plain)
                .font(.system(size: 17))
                .focused(focus)
                .autocorrectionDisabled()
                .accessibilityLabel(Text(prompt))
        }
        .padding(.horizontal, Space._4)
        .frame(height: Space._6 * 2)
    }
}
