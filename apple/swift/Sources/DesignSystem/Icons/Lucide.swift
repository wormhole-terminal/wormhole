// The Lucide icons the sidebar's marks, the panel's tabs and the title row use (`gpui-kit-assets`), drawn as strokes in their 24
// point box at any size. Their paths are generated (`LucideData.swift`).

import SwiftUI

public enum Lucide: Sendable, Hashable {
    case pin, link, note
    // The glyphs of the right panel's tabs and the branch chip.
    case gitCompare, gitBranch, gauge, folder, folderTree
    // What the panel's views draw (`LucidePanelData.swift`, generated).
    case chevronLeft, chevronRight, chevronDown, chevronsUpDown, x, plus, copy, check, refreshCw, slidersHorizontal, textWrap
    case externalLink, inbox, file, fileText, filePlus, info, shield, shieldCheck, triangleAlert, arrowUpRight, arrowRight
    case play, ellipsis, messageSquarePlus, ban
    case folderGit2, folderInput, squareTerminal, tag, chevronUp, settings, palette, archive, commit
    case search, caseSensitive, wholeWord, regex
}

/// An icon stroked in the foreground colour, its line width scaled with the box as the SVG's is.
public struct LucideView: View {
    let icon: Lucide
    let size: CGFloat

    public init(icon: Lucide, size: CGFloat) { (self.icon, self.size) = (icon, size) }
    private static let box: CGFloat = 24
    private static let line: CGFloat = 2

    public var body: some View {
        Canvas { canvas, area in
            let scale = area.width / Self.box
            var path = Path()
            for d in (Lucide.drawings[icon] ?? Lucide.panelDrawings[icon]) ?? [] { path.addPath(SVGPath.path(d), transform: CGAffineTransform(scaleX: scale, y: scale)) }
            canvas.stroke(path, with: .foreground,
                          style: StrokeStyle(lineWidth: Self.line * scale, lineCap: .round, lineJoin: .round))
        }
        .frame(width: size, height: size)
    }
}
