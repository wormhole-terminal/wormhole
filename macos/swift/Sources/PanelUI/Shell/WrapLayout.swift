// Children laid out in a row that wraps onto more lines, as chips do (GPUI's `flex_wrap`): a commit's ref chips and
// parent links in a narrow panel keep their whole names instead of being squeezed into one line.

import SwiftUI

struct WrapLayout: Layout {
    var spacing: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        arrange(subviews, width: proposal.width ?? .infinity).size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        for (subview, origin) in zip(subviews, arrange(subviews, width: bounds.width).origins) {
            subview.place(at: CGPoint(x: bounds.minX + origin.x, y: bounds.minY + origin.y), proposal: .unspecified)
        }
    }

    private func arrange(_ subviews: Subviews, width: CGFloat) -> (size: CGSize, origins: [CGPoint]) {
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
