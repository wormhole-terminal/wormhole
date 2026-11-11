// The profile's mark (`style::profile_avatar`): its letter on its colour, in the nav's select and on the Profile page.

import DesignSystem
import SidebarUI
import SwiftUI
import WorkspaceModel

struct ProfileChip: View {
    let letter: String
    /// A group colour's key; one this build does not know reads as the primary.
    let color: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(letter)
            .textRole(.caption, weight: .bold).foregroundStyle(tokens.textOnPrimary)
            .frame(width: Metric.titleGlyph + 4, height: Metric.titleGlyph + 4)
            .background(GroupColor(key: color).map { $0.mark(tokens).color } ?? tokens.primary, in: Circle())
            .accessibilityHidden(true)
    }
}
