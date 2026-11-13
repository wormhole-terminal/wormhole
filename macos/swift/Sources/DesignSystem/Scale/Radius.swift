// Corners and lines by role, from the theme's metrics (`metrics.radius_*`, `border_width`, ...), so a theme
// that rounds its chips into pills or draws no divider changes the whole UI. See `style/scale.rs`.

import SwiftUI

@MainActor
public enum Radius {
    private static var metrics: ThemeMetrics { ThemeStore.shared.tokens.metrics }

    /// Tiny elements: swatches, a small icon button's hover box.
    public static var sm: CGFloat { ScaleValue.of("radius.sm", metrics.radiusSm) }
    /// Anything chip-shaped: tabs, choice chips, badges, key caps.
    public static var chip: CGFloat { ScaleValue.of("radius.chip", metrics.radiusChip) }
    /// Controls and list rows: buttons, inputs, selects.
    public static var control: CGFloat { ScaleValue.of("radius.control", metrics.radius) }
    /// Cards, menus, popovers and other floating surfaces.
    public static var card: CGFloat { ScaleValue.of("radius.card", metrics.radiusLg) }
    /// The selection and hover of a row that stands inside a card (`CARD_ROW_RADIUS`): not the theme's.
    public static var row: CGFloat { ScaleValue.of("radius.row", 8) }
}

/// Line widths by role. `border` is a hairline, `divider` the line between columns (0 draws none).
@MainActor
public enum Rule {
    public static var border: CGFloat { ThemeStore.shared.tokens.metrics.borderWidth }
    public static var divider: CGFloat { ThemeStore.shared.tokens.metrics.dividerWidth }
    public static var focusRing: CGFloat { ThemeStore.shared.tokens.metrics.focusRingWidth }
}
