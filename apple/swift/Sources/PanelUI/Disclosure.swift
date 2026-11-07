// The chevron that shows whether a section is open (12 pt, muted): it turns a quarter as the section opens or
// folds, and swaps at once under Reduce Motion (`style::disclosure`).

import DesignSystem
import SwiftUI

struct Disclosure: View {
    let collapsed: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Image(systemName: "chevron.right")
            .font(.system(size: 12, weight: .medium))
            .foregroundStyle(tokens.textMuted)
            .rotationEffect(.degrees(collapsed ? 0 : 90))
            .animation(reduceMotion ? nil : .easeInOut(duration: 0.2), value: collapsed)
            .frame(width: Metric.lead, height: Metric.lead)
            .accessibilityHidden(true)
    }
}
