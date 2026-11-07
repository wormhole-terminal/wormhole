// What the person chose about how the window itself looks, and what the system overrules: the settings that
// are not theme tokens (`opacity`, `blur`, `liquid_glass`, the status bar, column dividers) beside Reduce
// Transparency and Increase Contrast, which make every surface opaque whatever the settings say
// (`settings::derive`, `platform::look`). One observable, owned by the app, read by every part of the window.

import AppKit
import DesignSystem
import Observation
import SwiftUI

@MainActor @Observable
public final class WindowLook {
    /// The surfaces' opacity over the blur, 0.3...1.0 (`opacity`, 0.8 by default).
    public var opacity: CGFloat = 0.8
    public var blur = true
    /// Liquid Glass under the sidebar and the panel (macOS 26 and later), off by default as in the GPUI app.
    public var liquidGlass = false
    /// The optional bar along the bottom, hidden by default.
    public var statusBar = false
    /// The lines between the columns (the Column dividers setting).
    public var columnDividers = true
    /// Reduce Transparency or Increase Contrast is on, as the theme store follows them live (it observes the
    /// system's own notifications), so a change redraws the window.
    var systemOpaque: Bool { ThemeStore.shared.reduceTransparency || ThemeStore.shared.increaseContrast }

    public init() {}

    /// What is painted: the asked opacity, or 1 when the system wants windows opaque.
    var effectiveOpacity: CGFloat { systemOpaque ? 1 : opacity }
    var isTranslucent: Bool { effectiveOpacity < 1 }
    /// Whether the blur material lies behind the surfaces.
    var blurShown: Bool { blur && isTranslucent }
    /// Glass needs the system's say-so too (`Settings::glass_on`).
    var glassOn: Bool { liquidGlass && !systemOpaque && Self.macHasGlass }

    /// `colour` at the window's opacity: every surface paints translucent over the blur.
    func surface(_ colour: Color) -> Color { colour.opacity(effectiveOpacity) }

    public static let macHasGlass: Bool = NSClassFromString("NSGlassEffectView") != nil

}
