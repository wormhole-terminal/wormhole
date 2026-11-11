// A panel glyph drawn as the GPUI app draws it, in Lucide (`IconName`), where it has one; the views name them by the symbol they
// used to ask for, so one table says which Lucide icon each stands for. A symbol with no Lucide twin is the system's.

import DesignSystem
import SwiftUI

struct PanelIcon: View {
    let name: String
    let size: CGFloat

    init(_ name: String, size: CGFloat) { (self.name, self.size) = (name, size) }

    var body: some View {
        if let icon = Self.lucide[name] { LucideView(icon: icon, size: size) } else { Image(systemName: name).font(.system(size: size)) }
    }

    /// The symbol a view asks for, and the Lucide icon the GPUI app draws there.
    static let lucide: [String: Lucide] = [
        "chevron.left": .chevronLeft, "chevron.right": .chevronRight, "chevron.down": .chevronDown,
        "chevron.up.chevron.down": .chevronsUpDown, "xmark": .x, "plus": .plus, "doc.on.doc": .copy, "checkmark": .check,
        "arrow.clockwise": .refreshCw, "slider.horizontal.3": .slidersHorizontal, "text.append": .textWrap,
        "arrow.up.forward.square": .externalLink, "tray.and.arrow.down": .inbox, "doc": .file, "doc.text": .fileText,
        "doc.badge.plus": .filePlus, "info.circle": .info, "shield": .shield, "checkmark.shield": .shieldCheck,
        "exclamationmark.triangle": .triangleAlert, "arrow.up.right": .arrowUpRight, "arrow.right": .arrowRight,
        "play.fill": .play, "ellipsis": .ellipsis, "text.bubble": .messageSquarePlus, "nosign": .ban,
        "arrow.triangle.branch": .gitBranch,
    ]
}
