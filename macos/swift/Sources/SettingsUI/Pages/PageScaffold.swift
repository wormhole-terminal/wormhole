// What every page shares: the title row (20 / bold, 38 high, with the page's action at its right), the intro
// under it, and the scrolling column of at most `columnWidth`. Scrolls to a row named by `revealed`.

import DesignSystem
import SwiftUI

struct PageScaffold<Action: View, Content: View>: View {
    let title: String
    var intro = ""
    /// A row to scroll into view (a search result was chosen); `clear` is called when its mark has had its moment.
    var revealed: String?
    var clearReveal: () -> Void = {}
    @ViewBuilder var action: Action
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: Space._4) {
                    VStack(alignment: .leading, spacing: Space._1) {
                        HStack(spacing: Space._4) {
                            Text(title).textRole(.title, weight: .bold).foregroundStyle(tokens.textPrimary)
                                .accessibilityAddTraits(.isHeader)
                            Spacer(minLength: 0)
                            action
                        }
                        .frame(height: Metric.titleRow)
                        if !intro.isEmpty {
                            Text(intro).textRole(.body).foregroundStyle(tokens.textMuted)
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
            .task(id: revealed) {
                guard let revealed else { return }
                withAnimation(Motion.layout) { proxy.scrollTo(revealed, anchor: .center) }
                // one-shot: the found row's highlight fades after 1.6 s
                try? await Task.sleep(for: .seconds(1.6))
                clearReveal()
            }
        }
    }
}
