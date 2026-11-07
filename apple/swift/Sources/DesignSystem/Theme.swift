// The one place colours, spacing, type and radii are named. Views never write a literal number or colour.
// Spacing is a 4-pt grid like the GPUI app's `style/`; colours follow its tokens (`settings.group`, ...).

import SwiftUI

public enum Space {
    public static let _1: CGFloat = 4
    public static let _2: CGFloat = 8
    public static let _3: CGFloat = 12
    public static let _4: CGFloat = 16
    public static let _6: CGFloat = 24
}

public enum Radius {
    public static let chip: CGFloat = 6
    public static let row: CGFloat = 8
    public static let card: CGFloat = 12
}

public enum Palette {
    public static let sidebarBackground = Color(nsColor: .underPageBackgroundColor)
    public static let hover = Color.primary.opacity(0.06)
    public static let selected = Color.accentColor.opacity(0.18)
    public static let divider = Color.primary.opacity(0.10)
    public static let textPrimary = Color.primary
    public static let textSecondary = Color.secondary
}
