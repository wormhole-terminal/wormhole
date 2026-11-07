// What only the app can do for the Settings pages: its buttons (the update check, a test notification, the default
// terminal, resetting the tips, the system's Developer Tools list), the installed apps and the system's fonts, and the
// programs let in, whose revoking ends what the hook server let them start. Core answers the rest (`RustSettingsHost`).

#if WITH_UNIFFI
    import AppKit
    import AuxBridge
    import PanesUI
    import SettingsBridge
    import SettingsModel
    import WormholeBindings

    @MainActor
    enum SettingsPlatform {
        static func make(app: AppController) -> RustSettingsHost.Platform {
            var platform = RustSettingsHost.Platform()
            platform.perform = { [weak app] action in app?.perform(action) }
            platform.fontFamilies = { monospacedFamilies() }
            platform.openApps = {
                SystemOpenIn().apps().map { SettingOption(value: .string($0.bundle), label: $0.name) }
            }
            platform.approvals = { [weak app] in desk(of: app)?.approvals() ?? [] }
            platform.revokeApproval = { [weak app] number, stored in
                if let hooks = app?.hooks { return hooks.revokeApproval(number, stored: stored) }
                // No server (a second launch, an unlinked socket): the desk alone takes it back.
                if stored { _ = desk(of: app)?.revokeStored(id: number) } else { _ = desk(of: app)?.revoke(id: number) }
            }
            return platform
        }

        private static func desk(of app: AppController?) -> AccessDesk? { (app?.aux.accessBackend as? FFIAccessBackend)?.desk }

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
            let host = settingsModel.host as? RustSettingsHost
            switch action {
            case .checkForUpdates: aux.showUpdates()
            case .sendTestNotification: notices?.sendTest(name: "Wormhole", settings: settingsNotice)
            case .openNotificationSettings: open("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
            // The system's list of apps that start programs without the first-run malware check (Privacy & Security).
            case .openDeveloperTools: open("x-apple.systempreferences:com.apple.preference.security?Privacy_DevTools")
            case .makeDefaultTerminal:
                Task { host?.problem(await DefaultTerminal.make(policy: policy)) }
            case .resetTips:
                do { _ = try resetTips() } catch { host?.problem(String(describing: error)) }
            default: break
            }
        }

        private func open(_ link: String) {
            if let url = URL(string: link) { NSWorkspace.shared.open(url) }
        }
    }
#endif

#if WITH_UNIFFI
    extension RustSettingsBackend: RefusalReporting {}
#endif
