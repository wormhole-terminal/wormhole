// What every page shares: the title row (20 / bold, 38 high, with the page's action at its right), the intro
// under it, and the scrolling column of at most `columnWidth`. Reveals a row by scrolling to its id.

import DesignSystem
import SwiftUI

struct PageScaffold<Action: View, Content: View>: View {
    let title: String
    var intro = ""
    @ViewBuilder var action: Action
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: Space._4) {
                    VStack(alignment: .leading, spacing: Space._1) {
                        HStack(spacing: Space._4) {
                            Text(title).font(.system(size: TextRole.title.size, weight: .bold)).foregroundStyle(tokens.textPrimary)
                                .accessibilityAddTraits(.isHeader)
                            Spacer(minLength: 0)
                            action
                        }
                        .frame(height: Metric.titleRow)
                        if !intro.isEmpty {
                            Text(intro).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textMuted)
                                .frame(maxWidth: SettingsMetric.introWidth, alignment: .leading)
                        }
                    }
                    content
                }
                .frame(maxWidth: SettingsMetric.columnWidth, alignment: .leading)
                .padding(.horizontal, Space._6 + Space._2).padding(.top, Space._4).padding(.bottom, Space._6 + Space._2)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollIndicators(.automatic)
            .background(tokens.surfaceWindow)
            .environment(\.revealScroll, { id in withAnimation(Motion.layout) { proxy.scrollTo(id, anchor: .center) } })
        }
    }
}

extension EnvironmentValues {
    /// Scrolls the open page to a row by its id (set by the page's scroll view).
    @Entry var revealScroll: (String) -> Void = { _ in }
}
