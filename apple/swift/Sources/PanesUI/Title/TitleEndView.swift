// One end of the row: its parts in order, the ones the row had room for, with the gaps and chevrons between
// them (`render_end`). The gap before a part is its own (`TitleLayout.gap`); a chevron is faint, with the
// breadcrumb's gap on each side.

import DesignSystem
import PanesModel
import SwiftUI

struct TitleEndView: View {
    let end: TitleEnd
    let plan: TitlePlan
    let actions: TitleActions
    let backend: any OpenInBackend
    let icon: (OpenInApp) -> NSImage?
    @Environment(\.tokens) private var tokens

    var body: some View {
        let drawn = zip(end.parts, end.chevrons).filter { plan.isDrawn($0.0) }
        HStack(spacing: 0) {
            ForEach(Array(drawn.enumerated()), id: \.element.0) { ix, item in
                let (part, chevron) = item
                let before = ix > 0 ? drawn[ix - 1].0 : nil
                let lead = TitleLayout.gap(before: before, part)
                if chevron {
                    Image(systemName: "chevron.right").font(.system(size: Metric.chevron, weight: .semibold))
                        .foregroundStyle(tokens.textFaint).frame(width: Metric.glyph)
                        .padding(.leading, lead).accessibilityHidden(true)
                }
                TitlePartView(part: part, plan: plan, actions: actions, backend: backend, icon: icon)
                    .padding(.leading, chevron ? TitleMetrics.crumbGap : lead)
                    .fixedSize(horizontal: part != .terminal, vertical: false)
                    .layoutPriority(part == .terminal ? -1 : 0)
            }
        }
        .padding(.trailing, TitleLayout.trailing(drawn.last?.0))
    }
}
