// An icon button for the title rows: a 28 pt target with a hover wash. Native buttons would draw bezels
// that fight the flat rows, so this is the one place the look is ours.

import DesignSystem
import SwiftUI

struct ChromeButton: View {
    let systemImage: String
    let label: String
    let action: @MainActor () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.body)
                .foregroundStyle(Palette.textSecondary)
                .frame(width: Chrome.hit, height: Chrome.hit)
                .background(hovering ? Palette.hover : .clear, in: RoundedRectangle(cornerRadius: Radius.chip))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}
