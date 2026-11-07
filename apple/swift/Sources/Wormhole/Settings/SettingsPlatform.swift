// What only the app can do for the Settings pages: its buttons, the installed apps and the system's fonts. Core answers
// the rest (`RustSettingsHost`). A button that needs a form of its own (a new profile) or a system the experiment is not
// (the default terminal) says so in the log instead of pretending.

#if WITH_UNIFFI
    import AppKit
    import PanesUI
    import SettingsBridge
    import SettingsModel

    @MainActor
    enum SettingsPlatform {
        static func make(app: AppController) -> RustSettingsHost.Platform {
            var platform = RustSettingsHost.Platform()
            platform.perform = { [weak app] action in app?.perform(action) }
            platform.fontFamilies = { monospacedFamilies() }
            platform.openApps = {
                SystemOpenIn().apps().map { SettingOption(value: .string($0.bundle), label: $0.name) }
            }
            return platform
        }

        /// The installed families that are fixed-pitch, which is what a terminal's font can be.
        static func monospacedFamilies() -> [String] {
            let manager = NSFontManager.shared
            return manager.availableFontFamilies.filter { family in
                (manager.availableMembers(ofFontFamily: family)?.first?.first as? String).flatMap { NSFont(name: $0, size: 12) }?
                    .isFixedPitch ?? false
            }
        }
    }

    extension AppController {
        /// A Settings button the core does not answer.
        func perform(_ action: SettingsAction) {
            switch action {
            case .checkForUpdates: aux.showUpdates()
            case .sendTestNotification: notices?.sendTest(name: "Wormhole", settings: settingsNotice)
            case .openNotificationSettings:
                if let url = URL(string: "x-apple.systempreferences:com.apple.Notifications-Settings.extension") {
                    NSWorkspace.shared.open(url)
                }
            case .makeDefaultTerminal, .newProfile, .deleteProfile, .resetTips, .openDeveloperTools:
                let line = "settings: \(action.rawValue) is not available in the experiment"
                FileHandle.standardError.write(Data((line + "\n").utf8))
                devctl?.append(line)
            default: break
            }
        }
    }
#endif

#if WITH_UNIFFI
    extension RustSettingsBackend: RefusalReporting {}
#endif
