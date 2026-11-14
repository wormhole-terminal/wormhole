// What the core supplies to the macOS edge, installed once at launch where the window is real: its keymap (every menu,
// recorder and tooltip reads it), its fuzzy matcher for the palette and its words (each module's one funnel reads them),
// and the `wh` command every terminal finds on its PATH (before the first terminal starts).
// Everything else keeps the dumped keymap, the Swift matcher and English.

import DesignSystem
import Foundation
import PlatformModel

#if WITH_UNIFFI
    import FilesBridge
    import PlatformBridge
    import UniFFISource
    import WordsBridge
#endif

enum PlatformSetup {
    @MainActor
    static func install(_ options: Options, launchLog: URL? = nil) {
        // A test instance never speaks; what it would say is kept for `a11y said`.
        Announcements.speaks = !options.background
        TestInstance.isBackground = options.background
        #if WITH_UNIFFI
            guard options.isReal else { return }
            recordLaunches(to: launchLog?.path)
            PlatformBridge.installKeymap()
            PlatformBridge.installPaletteMatcher()
            WordsBridge.install(tag: launchLanguage(options, environment: ProcessInfo.processInfo.environment))
            FilesBridge.installRules()
            installResizeRules()
            installCellRules()
            installCellShapes()
            CommandLineSetup.keepCurrent()
        #endif
    }

    /// The language a launch asks for by name; nil is the system's list (core's `negotiate`, which never gives the
    /// pseudo-language). GPUI takes a language by name from devctl alone, so a build without it cannot show `en-XA`
    /// (`app/language.rs:7-8`, `devctl/language.rs:17`). Here devctl is the control folder (`WORMHOLE_DEVCTL`), whose
    /// `language <tag>` is that command; `WORMHOLE_LANGUAGE` is its form at launch, so that every string built once shows the
    /// language too, and is not read by an instance without a control folder.
    static func launchLanguage(_ options: Options, environment: [String: String]) -> String? {
        guard options.devctl != nil else { return nil }
        return environment["WORMHOLE_LANGUAGE"].flatMap { $0.isEmpty ? nil : $0 }  // text: an environment variable name
    }

    /// The menu bar's rules: core's when real, the Swift port (over the dumped keymap) otherwise.
    static func menus(_ options: Options) -> any MenuProviding {
        #if WITH_UNIFFI
            if options.isReal { return PlatformBridge.makeMenus() }
        #endif
        return MenuBuilder()
    }
}
