// The Lucide icons the sidebar's marks use (`gpui-kit-assets`: pin, link-2, sticky-note), drawn as strokes in their 24
// point box at any size. Their paths are generated (`LucideData.swift`).

import SwiftUI

enum Lucide: Sendable, Hashable {
    case pin, link, note
}

/// An icon stroked in the foreground colour, its line width scaled with the box as the SVG's is.
struct LucideView: View {
    let icon: Lucide
    let size: CGFloat
    private static let box: CGFloat = 24
    private static let line: CGFloat = 2

    var body: some View {
        Canvas { canvas, area in
            let scale = area.width / Self.box
            var path = Path()
            for d in Lucide.drawings[icon] ?? [] { path.addPath(SVGPath.path(d), transform: CGAffineTransform(scaleX: scale, y: scale)) }
            canvas.stroke(path, with: .foreground,
                          style: StrokeStyle(lineWidth: Self.line * scale, lineCap: .round, lineJoin: .round))
        }
        .frame(width: size, height: size)
    }
}
