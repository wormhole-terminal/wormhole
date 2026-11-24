// Where the parts of an SF Symbol sit in its own box, measured, so a motion can cut a symbol into pieces the way
// `LucidePart` cuts a Lucide icon on its 24-unit grid. A symbol is not on a grid: every number here is a fraction of
// the symbol's layout box (its width and its height, origin top left), read from the symbol rendered at 400 pt in the
// regular weight (`NSImage(systemSymbolName:)` into a bitmap, scanned for alpha over 0.5). A symbol that changes
// weight keeps its shape's proportions; its box grows with the font, so the fractions stay.

import CoreGraphics

enum SymbolGeometry {
    /// The box the symbol's ink covers, by name: where a fill sweeping over it starts and ends.
    static let inks: [String: CGRect] = [
        "sidebar.left": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "sidebar.right": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "rectangle.leadinghalf.inset.filled": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "rectangle.trailinghalf.inset.filled": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "rectangle.split.2x1": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "rectangle.split.2x1.fill": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "rectangle.split.1x2": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "rectangle.split.1x2.fill": CGRect(x: 0.0891, y: 0.0814, width: 0.8200, height: 0.8372),
        "doc.on.doc": CGRect(x: 0.1000, y: 0.0641, width: 0.7980, height: 0.8737),
        "doc.on.doc.fill": CGRect(x: 0.1000, y: 0.0595, width: 0.7980, height: 0.8350),
        "checkmark": CGRect(x: 0.1144, y: 0.0866, width: 0.7689, height: 0.8267),
        "note.text": CGRect(x: 0.1020, y: 0.0814, width: 0.7939, height: 0.8372),
        "folder": CGRect(x: 0.0919, y: 0.0841, width: 0.8143, height: 0.8341),
        "folder.fill": CGRect(x: 0.0919, y: 0.0841, width: 0.8143, height: 0.8341),
        "lock": CGRect(x: 0.1459, y: 0.0773, width: 0.7082, height: 0.8455),
        "lock.open": CGRect(x: 0.0996, y: 0.0773, width: 0.8008, height: 0.8455),
        "lock.fill": CGRect(x: 0.1459, y: 0.0773, width: 0.7082, height: 0.8455),
        "pin": CGRect(x: 0.1678, y: 0.0684, width: 0.6623, height: 0.8650),
        "pin.fill": CGRect(x: 0.1678, y: 0.0684, width: 0.6623, height: 0.8650),
        "bell": CGRect(x: 0.0907, y: 0.0759, width: 0.8164, height: 0.8502),
        "bell.fill": CGRect(x: 0.0907, y: 0.0728, width: 0.8164, height: 0.8545),
        "plus": CGRect(x: 0.1182, y: 0.0916, width: 0.7612, height: 0.8193),
        "plus.circle.fill": CGRect(x: 0.0746, y: 0.0768, width: 0.8486, height: 0.8486),
        "xmark": CGRect(x: 0.1220, y: 0.0990, width: 0.7537, height: 0.8047),
        "xmark.circle": CGRect(x: 0.0746, y: 0.0768, width: 0.8486, height: 0.8486),
        "xmark.circle.fill": CGRect(x: 0.0746, y: 0.0768, width: 0.8486, height: 0.8486),
        "chevron.down": CGRect(x: 0.1136, y: 0.1341, width: 0.7705, height: 0.7318),
        "chevron.up": CGRect(x: 0.1136, y: 0.1341, width: 0.7705, height: 0.7318),
        "chevron.right": CGRect(x: 0.2577, y: 0.0856, width: 0.6564, height: 0.8289),
        "arrow.clockwise": CGRect(x: 0.0843, y: 0.0714, width: 0.8313, height: 0.8571),
        "clock.arrow.circlepath": CGRect(x: 0.0084, y: 0.0768, width: 0.9179, height: 0.8486),
        "plus.forwardslash.minus": CGRect(x: 0.1064, y: 0.0797, width: 0.7851, height: 0.8405),
        "gauge.with.dots.needle.67percent": CGRect(x: 0.0746, y: 0.0768, width: 0.8486, height: 0.8486),
        "tray.and.arrow.down": CGRect(x: 0.0891, y: 0.0692, width: 0.8200, height: 0.8635),
        "tray.and.arrow.down.fill": CGRect(x: 0.0891, y: 0.0692, width: 0.8200, height: 0.8635),
        "arrow.up.forward.square": CGRect(x: 0.1087, y: 0.0814, width: 0.7804, height: 0.8372),
        "arrow.up.forward.square.fill": CGRect(x: 0.1087, y: 0.0814, width: 0.7804, height: 0.8372),
        "info.circle": CGRect(x: 0.0746, y: 0.0768, width: 0.8486, height: 0.8486),
        "info.circle.fill": CGRect(x: 0.0746, y: 0.0768, width: 0.8486, height: 0.8486),
        "shield": CGRect(x: 0.1176, y: 0.0774, width: 0.7647, height: 0.8473),
        "shield.fill": CGRect(x: 0.1190, y: 0.0784, width: 0.7595, height: 0.8453),
        "play": CGRect(x: 0.1429, y: 0.0879, width: 0.8343, height: 0.8241),
        "play.fill": CGRect(x: 0.1429, y: 0.0879, width: 0.8343, height: 0.8241),
        "tag": CGRect(x: 0.0835, y: 0.0700, width: 0.8311, height: 0.8482),
        "tag.fill": CGRect(x: 0.0835, y: 0.0700, width: 0.8311, height: 0.8482),
        "gearshape": CGRect(x: 0.0717, y: 0.0738, width: 0.8566, height: 0.8545),
        "gearshape.fill": CGRect(x: 0.0732, y: 0.0752, width: 0.8536, height: 0.8518),
        "paintpalette": CGRect(x: 0.0647, y: 0.0779, width: 0.8688, height: 0.8463),
        "paintpalette.fill": CGRect(x: 0.0647, y: 0.0779, width: 0.8688, height: 0.8463),
        "folder.badge.gearshape": CGRect(x: 0.0831, y: 0.1302, width: 0.8754, height: 0.7939),
        "folder.fill.badge.gearshape": CGRect(x: 0.0831, y: 0.1302, width: 0.8754, height: 0.7939),
        "folder.badge.plus": CGRect(x: 0.0831, y: 0.1345, width: 0.8721, height: 0.7896),
        "folder.fill.badge.plus": CGRect(x: 0.0831, y: 0.1345, width: 0.8721, height: 0.7896),
    ]

    /// The ink's box, or the whole box for a symbol that was not measured.
    static func ink(_ name: String) -> CGRect { inks[name] ?? CGRect(x: 0, y: 0, width: 1, height: 1) }

    /// The frame of `sidebar.left`, `sidebar.right` and the `rectangle.split` symbols (one frame, one box): the inside of
    /// its strokes, and where the divider's strokes are. The outer corner's radius is 0.0909 of the width and the
    /// strokes are 0.0535 thick, which leaves the inside corner 0.0374.
    enum Frame {
        static let insideLeft = 0.1444, insideRight = 0.8538, insideTop = 0.1558, insideBottom = 0.8442
        /// `sidebar.left`'s divider (its strokes' left edge), and `sidebar.right`'s (their right edge).
        static let leftDividerStart = 0.3547, rightDividerEnd = 0.6453
        static let insideCorner = 0.0374
        /// The middle of a split: where the new half grows from.
        static let middle = 0.5
    }

    /// The front sheet of `doc.on.doc`, the one that is a plain rounded rectangle, and its corner's radius (of the width).
    /// Measured on `doc.on.doc.fill` (its box is taller) and moved to the outline's box by the pixels: both are 500 wide and
    /// their sheets start 133 and end 525 pixels from the top.
    static let copySheet = CGRect(x: 0.1000, y: 0.2367, width: 0.6060, height: 0.7010)
    static let copySheetCorner = 0.082

    /// The two lines of `note.text`, each padded by a hundredth so the stroke's round ends are inside.
    static let noteLines = [
        CGRect(x: 0.2696, y: 0.4365, width: 0.4526, height: 0.0735),
        CGRect(x: 0.2696, y: 0.6295, width: 0.3261, height: 0.0735),
    ]

    /// `clock.arrow.circlepath`'s circle: its centre and, as a share of the width, its radius.
    static let clockCentre = CGPoint(x: 0.5074, y: 0.5011)

    /// `gauge.with.dots.needle.*`: the hub the needle turns on, the needle's direction (degrees, clockwise from the right),
    /// and the box that holds the needle and none of the dots, a rectangle along that direction (centre, then length and
    /// width as shares of the width). The dots are at least 0.2 from the hub and the needle's tip is 0.26 out, in the gap
    /// between two of them, so the rectangle (0.07 wide to either side) takes the needle alone.
    enum Gauge {
        static let hub = CGPoint(x: 0.4989, y: 0.5011)
        static let needleDegrees = -46.66
        static let needleCentre = CGPoint(x: 0.5569, y: 0.4397)
        static let needleSize = CGSize(width: 0.38, height: 0.15)
    }
}
