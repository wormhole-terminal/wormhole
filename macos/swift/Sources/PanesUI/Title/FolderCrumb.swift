// The folder in the title row: its icon and the path, the parents muted and the last folder the text colour,
// cut in its middle where long (`PathSteps`). A click opens the folder in Finder, resting on it opens the hover
// card, a right-click the Finder-style path menu.

import DesignSystem
import PanesModel
import SwiftUI

struct FolderCrumb: View {
    let plan: TitlePlan
    let actions: TitleActions
    @State private var card = PathCardHover()
    @Environment(\.tokens) private var tokens
    @Environment(\.titleTinted) private var tinted

    var body: some View {
        // Under a group's tint the icon and the parents step up to the main text colour, so they keep their contrast
        // on it (`breadcrumb.rs` `soft`).
        let soft = tinted ? tokens.textPrimary : tokens.textMuted
        let text: PathText? = if case .step(let ix) = plan.fit.path { plan.steps[ix].text } else { nil }
        Button {
            card.close()
            actions.revealFolder()
        } label: {
            HStack(spacing: TitleMetrics.crumbGap) {
                Image(systemName: "folder").font(.system(size: Metric.glyph)).foregroundStyle(soft)
                if let text {
                    // The parents stand back; the folder itself is the text.
                    HStack(spacing: 0) {
                        Text(text.parents).foregroundStyle(soft)
                        Text(text.last).foregroundStyle(tokens.textPrimary).fontWeight(.medium)
                    }
                    .font(tokens.font(.label).weight(.regular))
                    .lineLimit(1).fixedSize()
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { card.hover(.path, $0) }
        .popover(isPresented: $card.shown, arrowEdge: .bottom) {
            PathCard(path: plan.facts.shownFolder ?? "", buttons: PathCardItem.allCases.map { $0.button(actions) }, hover: card)
        }
        .contextMenu { PathMenuItems(facts: plan.facts, actions: actions) }
        .accessibilityLabel(PanesText.openFolder(plan.facts.shownFolder ?? ""))
        .accessibilityAddTraits(.isButton)
    }
}

extension EnvironmentValues {
    /// The selected terminal's group has a colour, which is the title row's tint (`TitlePlan.tint` in the GPUI app).
    @Entry public var titleTinted = false
}
