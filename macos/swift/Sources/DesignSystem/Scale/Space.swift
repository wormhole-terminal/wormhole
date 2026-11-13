// The 4-pt grid of the GPUI app's `style/` (`_1 _2 _3 _4 _6`: no half steps). Views never write a number.

import SwiftUI

/// A value of the scales. In a debug build the inspector may have nudged it for this session (`Inspect/DesignOverrides`);
/// in a release build it is the standard and nothing else.
enum ScaleValue {
    @inline(__always)
    static func of(_ key: String, _ standard: CGFloat) -> CGFloat {
        #if DEBUG
            return DesignOverrides.shared.value(key, standard)
        #else
            return standard
        #endif
    }
}

public enum Space {
    public static var _1: CGFloat { ScaleValue.of("space._1", 4) }
    public static var _2: CGFloat { ScaleValue.of("space._2", 8) }
    public static var _3: CGFloat { ScaleValue.of("space._3", 12) }
    public static var _4: CGFloat { ScaleValue.of("space._4", 16) }
    public static var _6: CGFloat { ScaleValue.of("space._6", 24) }
}
