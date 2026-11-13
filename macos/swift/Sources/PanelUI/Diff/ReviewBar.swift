// The bar under a diff's rows while the terminal has comments (`render_review_bar`): how many, Clear, and Paste into
// Terminal, which pastes the review into the terminal's prompt for the person to read and send.

import DesignSystem
import SwiftUI

struct ReviewBar: View {
    let review: DiffReview
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            Text(review.review.summary).font(tokens.font(.body)).foregroundStyle(tokens.textMuted).lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            NoticeButton(title: panelText("review-clear", "Clear"), action: review.clear)
            NoticeButton(title: panelText("review-paste", "Paste into Terminal"), primary: true, action: review.paste)
                .help(panelText("review-paste-help", "Paste the comments into the terminal's prompt, for you to send"))
        }
        .padding(.leading, Space._4).padding(.trailing, Space._3)
        .frame(height: 40)
        .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
        .motionTransition(.move(edge: .bottom).combined(with: .opacity))
    }
}
