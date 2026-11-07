// The live theme. The person picks one theme for dark and one for light and a mode (system, light, dark);
// what shows follows from those and the system's Increase Contrast. Everything the UI draws reads
// `tokens` (directly, through `Palette`, or through `@Environment(\.tokens)`), and all of those read
// through Observation, so `setTheme` re-colours every open window with no view touched.
//
// Reduce Transparency is tracked but changes no token: the surfaces' opacity is the window's, not the theme's.

import AppKit
import Observation

@MainActor @Observable
public final class ThemeStore {
    public static let shared = ThemeStore()

    public let library: ThemeLibrary
    public var mode: ThemeMode = .system
    public private(set) var darkThemeID: String
    public private(set) var lightThemeID: String
    /// macOS's appearance, Increase Contrast and Reduce Transparency: followed live unless `follows` was false.
    public var systemAppearance: ThemeAppearance = .dark
    public var increaseContrast = false
    public var reduceTransparency = false

    @ObservationIgnored private let look: SystemLook?

    /// `follows: false` leaves the three system facts to the caller (tests, `--appearance`).
    public init(library: ThemeLibrary = .bundled, follows: Bool = true) {
        self.library = library
        darkThemeID = library.defaultDark
        lightThemeID = library.defaultLight
        look = follows ? SystemLook() : nil
        look?.onChange = { [weak self] snapshot in
            self?.systemAppearance = snapshot.appearance
            self?.increaseContrast = snapshot.increaseContrast
            self?.reduceTransparency = snapshot.reduceTransparency
        }
        if let snapshot = look?.snapshot {
            (systemAppearance, increaseContrast, reduceTransparency) =
                (snapshot.appearance, snapshot.increaseContrast, snapshot.reduceTransparency)
        }
    }

    /// The appearance showing now: the mode's, or macOS's under Match system.
    public var appearance: ThemeAppearance { mode.appearance(system: systemAppearance) }

    /// The theme showing now.
    public var theme: Theme {
        let id = appearance == .dark ? darkThemeID : lightThemeID
        return library.theme(id: id) ?? library.theme(id: library.defaultDark) ?? library.themes[0]
    }

    public var tokens: ThemeTokens { theme.tokens(increaseContrast: increaseContrast) }

    /// Makes `id` the pick for its own appearance, and shows it: a mode that would hide it moves to
    /// that appearance (the GPUI app only sets the pick; a menu here must visibly do something).
    public func setTheme(_ id: String) {
        guard let picked = library.theme(id: id) else { return }
        switch picked.appearance {
        case .dark: darkThemeID = id
        case .light: lightThemeID = id
        }
        if appearance != picked.appearance { mode = ThemeMode(rawValue: picked.appearance.rawValue) ?? mode }
    }
}
