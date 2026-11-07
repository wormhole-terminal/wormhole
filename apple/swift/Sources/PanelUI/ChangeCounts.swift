// "+9 −4" in green and red, with digits that do not jiggle when they change. Zero counts are left out.

import DesignSystem
import SwiftUI

struct ChangeCounts: View {
    let added: Int
    let removed: Int

    var body: some View {
        HStack(spacing: Space._1) {
            if added > 0 { Text("+\(added)").foregroundStyle(PanelColors.added) }
            if removed > 0 { Text("\u{2212}\(removed)").foregroundStyle(PanelColors.removed) }
        }
        .font(.subheadline.monospacedDigit())
        .accessibilityHidden(true)
    }
}
