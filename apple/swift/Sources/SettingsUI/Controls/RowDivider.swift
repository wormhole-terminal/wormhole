// The hairline between two rows of a card that a block lays out itself (the card's own rows get theirs from `SettingGroup`).

import DesignSystem
import SwiftUI

struct RowDivider: View {
    @Environment(\.tokens) private var tokens
    var body: some View { Rectangle().fill(tokens.settingsDivider).frame(height: tokens.metrics.borderWidth) }
}
