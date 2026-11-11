// The chevron that shows whether a section is open (12 pt, muted): it turns a quarter as the section opens or
// folds, and swaps at once under Reduce Motion (`style::disclosure`).

import DesignSystem
import SwiftUI

struct Disclosure: View {
    let collapsed: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        PanelIcon("chevron.right", size: 12)
            .foregroundStyle(tokens.textMuted)
            .rotationEffect(.degrees(collapsed ? 0 : 90))
            .animation(PanelMotion(reduced: reduceMotion).turn, value: collapsed)
            .frame(width: Metric.lead, height: Metric.lead)
            .accessibilityHidden(true)
    }
}
