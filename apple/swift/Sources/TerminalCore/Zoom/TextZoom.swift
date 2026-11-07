// The text size with ⌘+ ⌘- ⌘0: a view setting like Terminal.app's, points over the size in Settings, not saved
// (`window_actions::zoomed`, `view::motion::text_zoom`).

import Foundation

public enum TextZoom {
    /// What Settings allows for the terminal font (`FONT_SIZES`).
    public static let sizes: ClosedRange<Double> = 8...36

    /// The zoom (points over `base`) after one more `step`, kept within `sizes`.
    public static func zoomed(base: Double, zoom: Double, step: Double) -> Double {
        min(max(base + zoom + step, sizes.lowerBound), sizes.upperBound) - base
    }
}
