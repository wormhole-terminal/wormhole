// What the person chose about how the window itself looks, and what the system overrules: the settings that
// are not theme tokens (`opacity`, `blur`, `liquid_glass`, the status bar, column dividers) beside Reduce
// Transparency and Increase Contrast, which make every surface opaque whatever the settings say
// (`settings::derive`, `platform::look`). One observable, owned by the app, read by every part of the window.

import AppKit
import DesignSystem
import Observation
import SidebarUI
import SwiftUI
import WorkspaceModel

@MainActor @Observable
public final class WindowLook {
    /// The surfaces' opacity over the blur, 0.3...1.0 (`opacity`, 0.8 by default).
    public var opacity: CGFloat = 0.8
    public var blur = true
    /// Liquid Glass under the sidebar and the panel (macOS 26 and later), off by default as in the GPUI app.
    public var liquidGlass = false
    /// Zen mode: every window drawn with the Zen preset, for a while and saved nowhere (`settings::zen`). It is a layer over
    /// the person's own values below, which it never changes, so ending it brings back exactly what was there.
    public var zen: WindowShown?
    /// The optional bar along the bottom, hidden by default.
    public var statusBar: Bool {
        get { zen?.statusBar ?? ownStatusBar }
        set { ownStatusBar = newValue }
    }
    /// A window is dragged freely, its grid reflowing, instead of in whole cells of the grid (`smooth_resize`, off by default).
    public var smoothResize = false
    /// The minimap along the terminals' right edge instead of the scroll bar (`minimap`, off by default).
    public var minimap: Bool {
        get { zen?.minimap ?? ownMinimap }
        set { ownMinimap = newValue }
    }
    /// What the person set, whatever Zen mode shows meanwhile: what the menus' titles say.
    var ownStatusBar = false
    var ownMinimap = false
    /// A split's panes are each their own rounded box, apart (`split_gaps`, on by default).
    public var splitGaps = true
    /// Terminals as tabs under the title row instead of the sidebar's list (Show terminals in ▸ Tabs).
    public var tabsOnTop = false
    /// The lines between the columns (the Column dividers setting).
    public var columnDividers: Bool {
        get { zen?.columnDividers ?? ownColumnDividers }
        set { ownColumnDividers = newValue }
    }
    private var ownColumnDividers = true
    /// How the sidebar's list looks (density, cards, what rows show): Settings → Terminals & sidebar.
    public var sidebar = SidebarAppearance()
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
