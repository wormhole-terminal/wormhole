// A few lines of the sidebar to show what Wormhole is for: which agent is working, which needs input, which is
// done (`welcome_page::specimen`). Sample text and no behaviour; it is a picture of the real list.

import AuxModel
import DesignSystem
import SwiftUI

struct WelcomeSpecimen: View {
    @Environment(\.tokens) private var tokens

    private struct Sample: Identifiable {
        let id: String
        let kind: StatusKind
        let name: String
        let detail: String
        let selected = false
    }

    private let samples = [
        Sample(id: "working", kind: .working, name: "auth refactor", detail: "Editing middleware.rs…"),
        Sample(id: "attention", kind: .attention, name: "release notes", detail: "Needs input \u{B7} Approve git push?"),
        Sample(id: "done", kind: .done, name: "api tests", detail: "Finished \u{B7} 12m ago"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                SectionLabel("api-server")
                Spacer()
                CountBadge(samples.count)
            }
            .padding(.horizontal, Space._2)
            ForEach(samples) { sample in
                HStack(spacing: Space._2) {
                    StatusDot(sample.kind)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(sample.name).font(TextRole.body.font)
                        Text(sample.detail).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    }
                    Spacer(minLength: 0)
                }
                .padding(.horizontal, Space._2).padding(.vertical, Space._1)
                .frame(height: Metric.rowTwoLine)
                .background(sample.id == "working" ? tokens.sidebarRowSelected : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
            }
        }
        .padding(Space._1)
        .frame(maxWidth: .infinity)
        .background(tokens.surfacePanel, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
        .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusLg).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(AuxText.specimenCaption)
    }
}
