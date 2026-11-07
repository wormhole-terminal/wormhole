// The terminal's title row: the selected terminal's breadcrumb (folder › branch ● › tile and name) at the left
// end and its extras, Open in… and the splits at the right, laid out as the person arranged them and fitted to
// the width by the model's rules (`TitlePlanner`, the port of core's `fit_row`). The window gives it the room
// between its paddings; a right-click opens the row's menu of parts.

import DesignSystem
import PanesModel
import SwiftUI

public struct TitleRowView: View {
    let facts: TitleFacts
    let actions: TitleActions
    let backend: any OpenInBackend
    let icon: (OpenInApp) -> NSImage?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        facts: TitleFacts, actions: TitleActions = TitleActions(), backend: any OpenInBackend,
        icon: @escaping (OpenInApp) -> NSImage? = { _ in nil }
    ) {
        (self.facts, self.actions, self.backend, self.icon) = (facts, actions, backend, icon)
    }

    public var body: some View {
        GeometryReader { proxy in
            let plan = TitlePlanner.plan(facts, width: proxy.size.width, measure: TextMeasurer())
            HStack(spacing: 0) {
                TitleEndView(end: plan.ends.left, plan: plan, actions: actions, backend: backend, icon: icon)
                Spacer(minLength: 0)
                TitleEndView(end: plan.ends.right, plan: plan, actions: actions, backend: backend, icon: icon)
            }
            .frame(width: proxy.size.width, height: proxy.size.height)
            .clipped()
            .animation(reduceMotion ? Motion.fade : Motion.selection, value: plan)
        }
        .frame(height: Metric.titleRow - 2 * TitleMetrics.barInset)
        .contextMenu { TitleRowMenu(row: facts.row, actions: actions) }
    }
}
