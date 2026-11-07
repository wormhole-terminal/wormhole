// Lucide's `git-commit`, which the History tab wears in the GPUI app and the system's symbols have no match
// for: a ring with a line through it on each side, in Lucide's 24-unit grid scaled to the glyph.

import DesignSystem
import SwiftUI

struct CommitGlyph: View {
    let color: Color

    var body: some View {
        Shape().stroke(color, style: StrokeStyle(lineWidth: 2 * Self.unit, lineCap: .round))
            .frame(width: Metric.glyph, height: Metric.glyph)
            .accessibilityHidden(true)
    }

    private static let unit = Metric.glyph / 24

    private struct Shape: SwiftUI.Shape {
        func path(in rect: CGRect) -> Path {
            let u = rect.width / 24
            var path = Path(ellipseIn: CGRect(x: 9 * u, y: 9 * u, width: 6 * u, height: 6 * u))
            for (from, to) in [(1.05, 9.0), (15.0, 22.96)] {
                path.move(to: CGPoint(x: from * u, y: 12 * u))
                path.addLine(to: CGPoint(x: to * u, y: 12 * u))
            }
            return path
        }
    }
}
