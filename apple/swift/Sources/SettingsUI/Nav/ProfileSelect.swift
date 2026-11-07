// Whose settings these are (`render_profile_select`): the profile Wormhole is in, drawn as a quiet pop-up button
// that opens the Profile page, the nav's only entry for it, so on that page it wears a nav row's selected look.

import DesignSystem
import SwiftUI
import WordsModel

struct ProfileSelect: View {
    let name: String
    let letter: String
    let color: String
    let isSelected: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._2) {
                ProfileChip(letter: letter, color: color)
                Text(name).font(.system(size: TextRole.label.size, weight: isSelected ? .semibold : .regular)).lineLimit(1)
                Spacer(minLength: 0)
                Image(systemName: "chevron.up.chevron.down").font(.system(size: Metric.chevron, weight: .semibold))
            }
            .foregroundStyle(isSelected ? tokens.sidebarRowSelectedText : tokens.textMuted)
            .padding(.horizontal, Space._2).frame(height: Metric.control)
            .background(isSelected ? tokens.sidebarRowSelected : (hovering ? tokens.controlHover : tokens.controlBackground), in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel(Words.text("settings-section-profile", fallback: "Profile"))
        .accessibilityValue(name)
    }
}
