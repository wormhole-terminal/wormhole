// A piece of a Lucide icon on the icon's own 24-unit grid (`icons::part`), drawn over or under the whole icon where it
// belongs: the part of an icon that fills, draws itself or turns while it answers a click (`IconMotion`, `TabGlyph`).

import SwiftUI

/// Stroked in the foreground colour as the icons are, or filled with it.
struct LucidePart: View {
    enum Ink { case stroke, fill }

    let paths: [String]
    var ink = Ink.stroke
    /// Moved by this many grid units, so one shape serves for the pieces that differ only in place.
    var shift = CGSize.zero
    /// Only this rectangle of the grid shows.
    var cut: CGRect?
    /// Turned about a point of the grid.
    var turn: (degrees: Double, about: CGPoint)?

    var body: some View {
        Canvas { context, area in
            let scale = area.width / 24
            var drawn = Path()
            for d in paths { drawn.addPath(SVGPath.path(d)) }
            var move = CGAffineTransform.identity
            if let turn {
                move = move.translatedBy(x: turn.about.x, y: turn.about.y).rotated(by: turn.degrees * .pi / 180)
                    .translatedBy(x: -turn.about.x, y: -turn.about.y)
            }
            move = move.translatedBy(x: shift.width, y: shift.height)
            let drawing = drawn.applying(move.concatenating(CGAffineTransform(scaleX: scale, y: scale)))
            if let cut { context.clip(to: Path(CGRect(x: cut.minX * scale, y: cut.minY * scale, width: cut.width * scale, height: cut.height * scale))) }
            switch ink {
            case .fill: context.fill(drawing, with: .foreground)
            case .stroke: context.stroke(drawing, with: .foreground, style: StrokeStyle(lineWidth: 2 * scale, lineCap: .round, lineJoin: .round))
            }
        }
        .accessibilityHidden(true)
    }
}

extension LucidePart {
    /// The grid's own rectangle `left...right` by `top...bottom`.
    nonisolated static func cut(_ left: Double, _ top: Double, _ right: Double, _ bottom: Double) -> CGRect {
        CGRect(x: left, y: top, width: max(right - left, 0), height: bottom - top)
    }

    /// The pieces the motions lay over their icons: the front sheet and check of copy, the frame of the split icons,
    /// the dot and ring of a commit, a page, the gauge's arc and needle, the folder's body. Lines are made from a path.
    enum Piece {
        static func drawing(_ icon: Lucide, _ index: Int) -> String { Lucide.panelDrawings[icon]?[index] ?? "" }
        static var copySheet: LucidePart { LucidePart(paths: [drawing(.copy, 0)], ink: .fill) }
        static var check: LucidePart { LucidePart(paths: [drawing(.check, 0)]) }
        static func frameFill(cut: CGRect) -> LucidePart { LucidePart(paths: [drawing(.columns2, 0)], ink: .fill, cut: cut) }
        static func dot(at shift: CGSize = .zero) -> LucidePart { LucidePart(paths: [drawing(.commit, 0)], ink: .fill, shift: shift) }
        static var ring: LucidePart { LucidePart(paths: [drawing(.commit, 0)]) }
        static var page: LucidePart { LucidePart(paths: Array((Lucide.panelDrawings[.fileText] ?? []).prefix(2))) }
        static var gaugeArc: LucidePart { LucidePart(paths: ["M3.34 19a10 10 0 1 1 17.32 0"]) }
        static func gaugeNeedle(degrees: Double) -> LucidePart {
            LucidePart(paths: ["m12 14 4-4"], turn: (degrees, CGPoint(x: 12, y: 14)))
        }
        static var folderBody: LucidePart {
            LucidePart(
                paths: ["M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"],
                ink: .fill)
        }
        static func line(_ d: String, cut: CGRect, shift: CGSize = .zero) -> LucidePart { LucidePart(paths: [d], shift: shift, cut: cut) }
    }
}
