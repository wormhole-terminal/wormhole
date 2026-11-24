// A piece of an SF Symbol, the way `LucidePart` is a piece of a Lucide icon: the symbol drawn through a clip cut on its own
// box (`SymbolGeometry`), so one part of it can fill, draw itself or turn while the rest stays. A clip is a few shapes in
// fractions of the symbol's box; `without` shapes are cut out of what it keeps.

import SwiftUI

struct SymbolClip {
    enum Piece {
        case rect(CGRect)
        case rounded(CGRect, corners: RectangleCornerRadii)
        /// A circle: its centre and radius (a share of the width).
        case disc(CGPoint, radius: Double)
        /// A pie slice from `start` degrees (clockwise from the right) round by `sweep` degrees, wide enough to cover the box.
        case wedge(CGPoint, start: Double, sweep: Double)
        /// A rectangle turned by `degrees` about its own centre.
        case tilted(center: CGPoint, size: CGSize, degrees: Double)
    }

    var keeps: [Piece]
    var without: [Piece] = []

    /// What a fill growing from `edge` covers once it has `grown` (0 to 1) of the ink `box`: all of the box, margin included,
    /// when it is whole, so a fill that has grown shows whole.
    static func grown(from edge: SymbolMotion.Edge, _ grown: Double, over box: CGRect) -> SymbolClip {
        let margin = 0.1, share = min(max(grown, 0), 1)
        let full = CGRect(x: -margin, y: -margin, width: 1 + 2 * margin, height: 1 + 2 * margin)
        var rect = full
        if share < 1 {
            switch edge {
            case .leading: rect.size.width = box.minX + box.width * share - full.minX
            case .trailing: (rect.origin.x, rect.size.width) = (box.maxX - box.width * share, full.maxX - (box.maxX - box.width * share))
            case .top: rect.size.height = box.minY + box.height * share - full.minY
            case .bottom: (rect.origin.y, rect.size.height) = (box.maxY - box.height * share, full.maxY - (box.maxY - box.height * share))
            }
        }
        return SymbolClip(keeps: [.rect(rect)])
    }
}

/// The clip as a shape in the box it is laid on (an even-odd fill: what is cut out of what is kept leaves a hole).
struct SymbolClipShape: Shape {
    let clip: SymbolClip

    func path(in rect: CGRect) -> Path {
        var path = Path()
        for piece in clip.keeps + clip.without { path.addPath(Self.path(of: piece, in: rect)) }
        return path
    }

    private static func path(of piece: SymbolClip.Piece, in box: CGRect) -> Path {
        let (w, h) = (box.width, box.height)
        func scaled(_ r: CGRect) -> CGRect { CGRect(x: box.minX + r.minX * w, y: box.minY + r.minY * h, width: r.width * w, height: r.height * h) }
        func point(_ p: CGPoint) -> CGPoint { CGPoint(x: box.minX + p.x * w, y: box.minY + p.y * h) }
        switch piece {
        case .rect(let r): return Path(scaled(r))
        case .rounded(let r, let corners):
            let radii = RectangleCornerRadii(
                topLeading: corners.topLeading * w, bottomLeading: corners.bottomLeading * w,
                bottomTrailing: corners.bottomTrailing * w, topTrailing: corners.topTrailing * w)
            return Path(roundedRect: scaled(r), cornerRadii: radii)
        case .disc(let c, let radius):
            let centre = point(c)
            return Path(ellipseIn: CGRect(x: centre.x - radius * w, y: centre.y - radius * w, width: 2 * radius * w, height: 2 * radius * w))
        case .wedge(let c, let start, let sweep):
            let centre = point(c)
            var path = Path()
            path.move(to: centre)
            path.addArc(center: centre, radius: hypot(w, h), startAngle: .degrees(start), endAngle: .degrees(start + sweep), clockwise: false)
            path.closeSubpath()
            return path
        case .tilted(let c, let size, let degrees):
            let centre = point(c)
            let turn = CGAffineTransform(translationX: centre.x, y: centre.y).rotated(by: degrees * .pi / 180)
            return Path(CGRect(x: -size.width * w / 2, y: -size.height * w / 2, width: size.width * w, height: size.height * w)).applying(turn)
        }
    }
}

/// An SF Symbol, whole or through a clip, in the weight and size the app draws it.
struct SymbolPart: View {
    let name: String
    let size: CGFloat
    let weight: Font.Weight
    var clip: SymbolClip?

    var body: some View {
        Image(systemName: name).font(.system(size: size, weight: weight))
            .mask { if let clip { SymbolClipShape(clip: clip).fill(style: FillStyle(eoFill: true)) } else { Rectangle() } }
    }
}

extension View {
    /// A shape filled in the foreground colour behind this symbol, on the symbol's box (a strip or a sheet under it).
    func symbolFill(_ clip: SymbolClip, opacity: Double = 1) -> some View {
        background { SymbolClipShape(clip: clip).fill(.foreground, style: FillStyle(eoFill: true)).opacity(opacity) }
    }
}
