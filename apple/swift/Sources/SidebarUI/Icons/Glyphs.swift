// The small drawings the tile and the rows use, as shapes in the GPUI app's own 16 and 24 point boxes
// (`assets/glyphs/*.svg`, Lucide's chevron, plus and x), so a stroke here is the stroke there at any size.
// Each strokes with `Glyph.stroke`, which scales the SVG's line width with the box.

import SwiftUI

enum Glyph: Sendable {
    case terminal, needsYouBar, check, failed, bell, chevron, plus, close

    /// The box the drawing was made in.
    var box: CGFloat {
        switch self {
        case .chevron, .plus, .close: 24
        default: 16
        }
    }

    /// The SVG's `stroke-width`.
    var lineWidth: CGFloat {
        switch self {
        case .terminal, .failed, .bell: 1.5
        case .needsYouBar: 3
        case .check, .chevron, .plus, .close: 2
        }
    }

    func path(in rect: CGRect) -> Path {
        let k = rect.width / box
        func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: rect.minX + x * k, y: rect.minY + y * k) }
        var path = Path()
        switch self {
        case .terminal:
            path.addRoundedRect(in: CGRect(origin: p(1.5, 2.5), size: CGSize(width: 13 * k, height: 11 * k)),
                                cornerSize: CGSize(width: 1.5 * k, height: 1.5 * k))
            path.move(to: p(4.5, 6)); path.addLine(to: p(6.5, 8)); path.addLine(to: p(4.5, 10))
            path.move(to: p(8.5, 10)); path.addLine(to: p(11.5, 10))
        case .needsYouBar:
            // The bar only; the dot is `NeedsYouDot`, filled.
            path.move(to: p(8, 2.5)); path.addLine(to: p(8, 8))
        case .check:
            path.move(to: p(3.5, 8.5)); path.addLine(to: p(6.5, 11.5)); path.addLine(to: p(12.5, 4.5))
        case .failed:
            path.addEllipse(in: CGRect(origin: p(2, 2), size: CGSize(width: 12 * k, height: 12 * k)))
            path.move(to: p(6, 6)); path.addLine(to: p(10, 10))
            path.move(to: p(10, 6)); path.addLine(to: p(6, 10))
        case .bell:
            path.move(to: p(4, 11)); path.addLine(to: p(4, 7))
            path.addArc(center: p(8, 7), radius: 4 * k, startAngle: .degrees(180), endAngle: .degrees(0), clockwise: false)
            path.addLine(to: p(12, 11)); path.addLine(to: p(13, 12.5)); path.addLine(to: p(3, 12.5)); path.closeSubpath()
            path.move(to: p(6.5, 14)); path.addLine(to: p(9.5, 14))
        case .chevron:
            path.move(to: p(9, 18)); path.addLine(to: p(15, 12)); path.addLine(to: p(9, 6))
        case .plus:
            path.move(to: p(5, 12)); path.addLine(to: p(19, 12))
            path.move(to: p(12, 5)); path.addLine(to: p(12, 19))
        case .close:
            path.move(to: p(18, 6)); path.addLine(to: p(6, 18))
            path.move(to: p(6, 6)); path.addLine(to: p(18, 18))
        }
        return path
    }
}

/// A glyph drawn at `size` points, stroked in the foreground colour with its SVG's line width scaled to fit.
struct GlyphView: View {
    let glyph: Glyph
    let size: CGFloat

    var body: some View {
        GlyphShape(glyph: glyph)
            .stroke(style: StrokeStyle(lineWidth: glyph.lineWidth * size / glyph.box, lineCap: .round, lineJoin: .round))
            .frame(width: size, height: size)
    }
}

struct GlyphShape: Shape {
    let glyph: Glyph
    func path(in rect: CGRect) -> Path { glyph.path(in: rect) }
}

/// The "!" of a needs-you badge: a bar and a dot, drawn rather than set in a font, whose "!" sits on its own bearings.
struct NeedsYouMark: View {
    let size: CGFloat

    var body: some View {
        ZStack {
            GlyphView(glyph: .needsYouBar, size: size)
            Circle().frame(width: 3.5 * size / 16, height: 3.5 * size / 16).offset(y: 5 * size / 16)
        }
        .frame(width: size, height: size)
    }
}
