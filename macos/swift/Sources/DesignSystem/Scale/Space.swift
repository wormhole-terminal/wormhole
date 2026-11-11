// The 4-pt grid of the GPUI app's `style/` (`_1 _2 _3 _4 _6`: no half steps). Views never write a number.

import SwiftUI

public enum Space {
    public static let _1: CGFloat = 4
    public static let _2: CGFloat = 8
    public static let _3: CGFloat = 12
    public static let _4: CGFloat = 16
    public static let _6: CGFloat = 24
}
