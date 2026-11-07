// The status tile's proportions, ported from `style/tile/` (`SQUIRCLE`, `FACE_SHARE`, `SHEEN_REACH`, the badge and
// ring constants). They are the tile's own drawing, not spacing or type, so they are named here and nowhere else.

import SwiftUI

enum TileGeometry {
    /// The corner as a share of the size: rounder than a control, the way macOS draws an app icon.
    static let corner: CGFloat = 0.3
    /// The terminal symbol as a share of the tile.
    static let face: CGFloat = 0.5
    /// How far down the tile the sheen reaches, from its full strength at the top.
    static let sheenReach: CGFloat = 0.55
    /// How far the light and the amber edge sit outside the tile's own edge.
    static let ringOut: CGFloat = 1
    /// The light's line in a tile and in one below `thinBelow`.
    static let stroke: CGFloat = 2
    static let strokeThin: CGFloat = 1.5
    static let thinBelow: CGFloat = 28
    /// The light: the share of the edge it covers, the steps its tail fades in, the track left under it.
    static let span = 0.32
    static let steps = 12
    static let track = 0.22
    /// The corner badge, in a tile and in a thin one, its cut-out rim, and how far it hangs past the corner.
    static let badge: CGFloat = 14
    static let badgeThin: CGFloat = 10
    static let rim: CGFloat = 2
    static let hang: CGFloat = 3

    static func line(_ size: CGFloat) -> CGFloat { size < thinBelow ? strokeThin : stroke }
    static func badgeSize(_ tile: CGFloat) -> CGFloat { tile < thinBelow ? badgeThin : badge }
}
