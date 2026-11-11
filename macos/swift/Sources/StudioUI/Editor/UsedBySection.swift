// What draws with the selected token: the gallery's components, outlined there while it is selected
// (`used_by_section`). A token no component uses directly says that what follows it does.

import DesignSystem
import StudioModel
import SwiftUI

struct UsedBySection: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let uses = document.uses(of: token)
        EditorSection {
            HStack {
                SectionLabel(StudioWords.sectionUsedBy)
                Text("\(uses.count)").font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
            }
            if uses.isEmpty {
                Text(StudioWords.usedByNone).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            } else {
                FlowLayout(spacing: Space._1) {
                    ForEach(uses) { use in
                        Text(use.label).font(TextRole.caption.font).padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
                            .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                    }
                }
                Text(StudioWords.usedByOutlined).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
            }
        }
    }
}

/// Wraps its children onto lines, as chips do.
struct FlowLayout: Layout {
    var spacing: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        layout(subviews, width: proposal.width ?? .infinity).size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        for (subview, point) in zip(subviews, layout(subviews, width: bounds.width).origins) {
            subview.place(at: CGPoint(x: bounds.minX + point.x, y: bounds.minY + point.y), proposal: .unspecified)
        }
    }

    private func layout(_ subviews: Subviews, width: CGFloat) -> (size: CGSize, origins: [CGPoint]) {
        var origins: [CGPoint] = []
        var (x, y, line, widest) = (CGFloat(0), CGFloat(0), CGFloat(0), CGFloat(0))
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x > 0, x + size.width > width { (x, y, line) = (0, y + line + spacing, 0) }
            origins.append(CGPoint(x: x, y: y))
            (x, line, widest) = (x + size.width + spacing, max(line, size.height), max(widest, x + size.width))
        }
        return (CGSize(width: widest, height: y + line), origins)
    }
}
