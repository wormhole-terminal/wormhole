// The live theme. The person picks one theme for dark and one for light and a mode (system, light, dark);
// what shows follows from those, the primary and branch picks, the window's look and the system's Increase
// Contrast and Reduce Transparency. Everything the UI draws reads `tokens` (directly, through `Palette`, or
// through `@Environment(\.tokens)`), and all of those read through Observation, so any change here re-colours
// every open window with no view touched. How `tokens` is made is in `ThemeStore+Resolve.swift`.

import AppKit
import Observation

@MainActor @Observable
public final class ThemeStore {
    /// Nothing of it is saved here: the choices are the settings' (`ThemeSync`), each instance's own, so one
    /// instance never re-themes another.
    public static let shared = ThemeStore()

    public let library: ThemeLibrary
    public var settings: ThemeSettings {
        didSet {
            refresh()
        }
    }
    /// macOS's appearance, Increase Contrast and Reduce Transparency: followed live unless `follows` was false.
    public var systemAppearance: ThemeAppearance = .dark { didSet { refresh() } }
    public var increaseContrast = false { didSet { refresh() } }
    /// Every window opaque, as in the GPUI app: no surface lets the desktop through and the text stands as the
    /// opaque rules made it.
    public var reduceTransparency = false { didSet { refresh() } }
    /// Reduce Motion: the chrome's animations hold still or fade (`\.reduceMotion`, set at each window's root by `.themed()`).
    public var reduceMotion = false
    /// The Borders setting: the lines of `border`, `card.edge` and `settings.divider` are drawn (`ui.line`); Increase
    /// Contrast keeps them whatever it says, as in the GPUI app.
    public var showsBorders = true { didSet { if showsBorders != oldValue { refresh() } } }
    /// The interface font picked in Settings → Fonts: every type role reads it through the tokens' metrics.
    public var interfaceFont = InterfaceFont() { didSet { if interfaceFont != oldValue { refresh() } } }
    /// "Also translucent" for the Settings, Studio and Session Hosts windows, and the main window's opacity and blur they borrow.
    public var secondary = SecondaryLook()
    /// The folder of the person's theme files (read only). Needs a `resolver`.
    public var themesDirectory: URL? { didSet { reloadUserThemes() } }
    /// Rust's loader and token rules at run time; without one, user themes, colour picks and opacities other
    /// than the default are not available and `problem` says so.
    public var resolver: (any ThemeResolving)? { didSet { reloadUserThemes() } }

    public internal(set) var userThemes: [ThemeSummary] = []
    public internal(set) var theme: Theme
    public internal(set) var tokens: ThemeTokens
    /// Why the theme shown is not the one asked for, if it isn't.
    public internal(set) var problem: String?
    /// A theme shown in place of the picks while the Theme Studio edits it (or previews a saved version):
    /// every window follows, and clearing it puts the person's pick back.
    public var preview: Theme? { didSet { refresh() } }

    @ObservationIgnored private let look: SystemLook?

    /// `follows: false` leaves the three system facts to the caller (tests, `--appearance`).
    init(library: ThemeLibrary = .bundled, follows: Bool = true) {
        self.library = library
        settings = ThemeSettings(library: library)
        let start = library.theme(id: library.defaultDark) ?? library.themes[0]
        (theme, tokens) = (start, start.tokens())
        look = follows ? SystemLook() : nil
        look?.onChange = { [weak self] snapshot in
            self?.systemAppearance = snapshot.appearance
            self?.increaseContrast = snapshot.increaseContrast
            self?.reduceTransparency = snapshot.reduceTransparency
            self?.reduceMotion = snapshot.reduceMotion
        }
        if let snapshot = look?.snapshot {
            (systemAppearance, increaseContrast, reduceTransparency, reduceMotion) =
                (snapshot.appearance, snapshot.increaseContrast, snapshot.reduceTransparency, snapshot.reduceMotion)
        }
        refresh()
    }

    public convenience init(follows: Bool) { self.init(library: .bundled, follows: follows) }

    public var mode: ThemeMode {
        get { settings.mode }
        set { settings.mode = newValue }
    }

    /// The appearance showing now: the preview's, else the mode's, or macOS's under Match system.
    public var appearance: ThemeAppearance { preview?.appearance ?? settings.mode.appearance(system: systemAppearance) }
    public var darkThemeID: String { settings.darkTheme }
    public var lightThemeID: String { settings.lightTheme }

    /// Makes `id` (a bundled slug, or a user theme's id) the pick for its own appearance, and shows it: a
    /// mode that would hide it moves to that appearance (the GPUI app only sets the pick; a menu here must
    /// visibly do something).
    public func setTheme(_ id: String) {
        guard let picked = library.theme(id: id)?.appearance ?? userThemes.first(where: { $0.id == id })?.appearance
        else { return }
        var next = settings
        if picked == .dark { next.darkTheme = id } else { next.lightTheme = id }
        if next.mode.appearance(system: systemAppearance) != picked {
            next.mode = ThemeMode(rawValue: picked.rawValue) ?? next.mode
        }
        settings = next
    }
}
