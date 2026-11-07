// The diagonal hatch of a filler, where one side of a split has no line or a column has nothing to show:
// 45° stripes in `diff.filler`, three to a row so they run on unbroken from one filler row to the next.

import DesignSystem
import SwiftUI

struct DiffHatch: View {
    let rowHeight: CGFloat
    @Environment(\.tokens) private var tokens

    var body: some View {
        Canvas { context, size in
            let period = rowHeight / 3
            var path = Path()
            var x = -size.height
            while x < size.width {
                path.move(to: CGPoint(x: x, y: size.height))
                path.addLine(to: CGPoint(x: x + size.height, y: 0))
                x += period
            }
            context.stroke(path, with: .color(tokens.diffFiller), lineWidth: 1)
        }
        .accessibilityHidden(true)
    }
}
