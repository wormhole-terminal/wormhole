// The Usage tab (`features/usage/view.rs`): a card for each agent that has reported, or a note on how to get reports. The
// words that move with time ("resets 34m", "Updated 3m ago") are made from `now`, which the timeline moves on at each
// minute: the same as the app's next-change timer, since the words are in whole minutes.

import DesignSystem
import Foundation
import PanelModel
import SwiftUI

public struct UsageView: View {
    let store: UsageStore
    /// A fixed clock for screenshots and tests; nil follows the real one.
    var fixedNow: Int?
    @Environment(\.tokens) private var tokens

    public init(store: UsageStore, now: Int? = nil) {
        self.store = store
        fixedNow = now
    }

    public var body: some View {
        PanelScroll {
            if store.reports.isEmpty {
                PanelCard {
                    VStack(alignment: .leading, spacing: Space._3) {
                        Text(panelText("usage-empty-1", "Claude and Codex plan limits, and Gemini response tokens, show here as you use them in Wormhole."))
                        Text(panelText("usage-empty-2", "Enable or update their integrations in Settings \u{2192} Coding agents. Plan limits need a supported subscription; API-key logins may not report them."))
                    }
                    .font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                    .padding(.horizontal, PanelMetrics.inset).padding(.vertical, Space._2).frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                TimelineView(.everyMinute) { timeline in
                    let now = fixedNow ?? Int(timeline.date.timeIntervalSince1970)
                    ForEach(store.reports) { report in UsageCard(report: report, now: now) }
                }
            }
        }
    }
}
