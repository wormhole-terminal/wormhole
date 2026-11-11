// What the core supplies to the macOS edge, installed once at launch where the window is real: its keymap (every menu,
// recorder and tooltip reads it), its fuzzy matcher for the palette and its words (each module's one funnel reads them),
// and the `wh` command every terminal finds on its PATH (before the first terminal starts).
// Everything else keeps the dumped keymap, the Swift matcher and English.

import DesignSystem
import PlatformModel

#if WITH_UNIFFI
    import FilesBridge
    import PlatformBridge
    import UniFFISource
    import WordsBridge
#endif

enum PlatformSetup {
    @MainActor
    static func install(_ options: Options) {
        // A test instance never speaks; what it would say is kept for `a11y said`.
        Announcements.speaks = !options.background
        #if WITH_UNIFFI
            guard options.isReal else { return }
            recordLaunches(to: Launcher.diverted?.path)
            PlatformBridge.installKeymap()
            PlatformBridge.installPaletteMatcher()
            WordsBridge.install()
            FilesBridge.installRules()
            installResizeRules()
            installCellRules()
            installCellShapes()
            CommandLineSetup.keepCurrent()
        #endif
    }

    /// The menu bar's rules: core's when real, the Swift port (over the dumped keymap) otherwise.
    static func menus(_ options: Options) -> any MenuProviding {
        #if WITH_UNIFFI
            if options.isReal { return PlatformBridge.makeMenus() }
        #endif
        return MenuBuilder()
    }
}
