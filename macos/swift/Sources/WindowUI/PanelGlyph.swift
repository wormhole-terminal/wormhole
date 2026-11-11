// Lucide's `panel-left` / `panel-right` (the glyphs the GPUI app's toggles use), drawn here so the column's
// strip can fill and drain: full while its column is in, following the slide's own curve. Everything is in
// Lucide's 24-unit grid, scaled to the glyph's size (`style/icon_motion/strip.rs` has the same numbers).

import SwiftUI

struct PanelGlyph: View {
    enum Side { case left, right }

    let side: Side
    /// How full the strip is, 0...1.
    let fill: CGFloat
    let color: Color

    var body: some View {
        ZStack {
            Strip(side: side, fill: fill).fill(color)
            Frame(side: side).stroke(color, style: StrokeStyle(lineWidth: 2 * Self.unit, lineCap: .round, lineJoin: .round))
        }
        .frame(width: Chrome.glyph, height: Chrome.glyph)
        .accessibilityHidden(true)
    }

    /// One Lucide unit in points.
    static let unit = Chrome.glyph / 24

    private struct Frame: Shape {
        let side: Side

        func path(in rect: CGRect) -> Path {
            let u = rect.width / 24
            var path = Path(roundedRect: CGRect(x: 3 * u, y: 3 * u, width: 18 * u, height: 18 * u), cornerRadius: 2 * u)
            let x = (side == .left ? 9 : 15) * u
            path.move(to: CGPoint(x: x, y: 3 * u))
            path.addLine(to: CGPoint(x: x, y: 21 * u))
            return path
        }
    }

    /// The strip between the frame and the divider, whose outer corners follow the frame's.
    private struct Strip: Shape {
        let side: Side
        var fill: CGFloat

        var animatableData: CGFloat {
            get { fill }
            set { fill = newValue }
        }

        func path(in rect: CGRect) -> Path {
            let u = rect.width / 24
            let width = 6 * u * fill
            guard width > 0 else { return Path() }
            let x = side == .left ? 3 * u : 21 * u - width
            let corner = 2 * u
            let radii = side == .left
                ? RectangleCornerRadii(topLeading: corner, bottomLeading: corner)
                : RectangleCornerRadii(bottomTrailing: corner, topTrailing: corner)
            return Path(roundedRect: CGRect(x: x, y: 3 * u, width: width, height: 18 * u), cornerRadii: radii)
        }
    }
}
