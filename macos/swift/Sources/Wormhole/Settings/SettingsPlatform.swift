// What only the app can do for the Settings pages: its buttons (the update check, a test notification, the default
// terminal, resetting the tips, the system's Developer Tools list), the installed apps and the system's fonts, and the
// programs let in, whose revoking ends what the hook server let them start. Core answers the rest (`RustSettingsHost`).

#if WITH_UNIFFI
    import AppKit
    import AuxBridge
    import AuxModel
    import DesignSystem
    import FilesBridge
    import PanesUI
    import PlatformModel
    import PlatformUI
    import SettingsBridge
    import SettingsModel
    import WormholeBindings

    @MainActor
    enum SettingsPlatform {
        static func make(app: AppController) -> RustSettingsHost.Platform {
            var platform = RustSettingsHost.Platform()
            platform.perform = { [weak app] action in app?.perform(action) }
            platform.bundledFonts = { [weak app] in
                (app?.options.fontDirectory ?? Options.findBundledFonts()).map { bundledFiles(in: $0) } ?? []
            }
            platform.isDefaultTerminal = { DefaultTerminal.isDefault() }
            platform.openApps = {
                SystemOpenIn().apps().map { SettingOption(value: .string($0.bundle), label: $0.name) }
            }
            platform.approvals = { [weak app] in desk(of: app)?.approvals() ?? [] }
            platform.revokeApproval = { [weak app] number, stored in
                if let hooks = app?.hooks, let profile = app?.windowsHost.profile {
                    return hooks.revokeApproval(number, stored: stored, profile: profile)
                }
                // No server (a second launch, an unlinked socket): the desk alone takes it back.
                if stored { _ = desk(of: app)?.revokeStored(id: number) } else { _ = desk(of: app)?.revoke(id: number) }
            }
            // The Project folders page shares the Project tab's service, so trust given here is what the tab shows.
            // Asked while Settings is made, which can be before the first window exists.
            platform.projects = { [weak app] in (app?.sessions.first?.panelBackends.project as? RustProjectBackend)?.service }
            platform.projectFolders = { [weak app] in app?.hub.state.all.compactMap(\.folder) ?? [] }
            platform.openTerminal = { [weak app] in app?.hub.send(.newTerminalIn($0)) }
            platform.profileCreated = { [weak app] in app?.switchProfile(to: $0) }
            platform.willDeleteProfile = { [weak app] in app?.end(profile: $0) }
            // Owner authentication is Touch ID in a real run and WORMHOLE_AUTH's answer in a test instance.
            platform.ownerAvailable = { [weak app] in
                guard let policy = app?.policy else { return true }
                return (policy.touchesSystem ? LocalAuthenticator() as any Authenticating : ScriptedAuthenticator()).canAuthenticate()
            }
            platform.owner = { [weak app] in
                guard let policy = app?.policy else { return nil }
                return OwnerAuthentication(policy.touchesSystem ? LocalAuthenticator() : ScriptedAuthenticator())
            }
            return platform
        }

        private static func desk(of app: AppController?) -> AccessDesk? { (app?.aux.accessBackend as? FFIAccessBackend)?.desk }

        /// JetBrains Mono's four files in the folder that holds them (`FontSpec.jetBrainsMono`).
        private static func bundledFiles(in directory: URL) -> [URL] {
            ["Regular", "Bold", "Italic", "BoldItalic"].map { directory.appendingPathComponent("JetBrainsMono-\($0).ttf") } // text: font file names
        }
    }

    extension AppController {
        /// A Settings button the core does not answer.
        func perform(_ action: SettingsAction) {
            let host = settingsModel.host as? RustSettingsHost
            switch action {
            case .checkForUpdates: aux.showUpdates()
            case .sendTestNotification: notices?.sendTest(name: AuxText.brand, settings: settingsNotice)
            case .openNotificationSettings: open("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
            // The system's list of apps that start programs without the first-run malware check (Privacy & Security).
            case .openDeveloperTools: open("x-apple.systempreferences:com.apple.preference.security?Privacy_DevTools")
            case .makeDefaultTerminal:
                Task { host?.problem(await DefaultTerminal.make(policy: policy)) }
            case .resetTips:
                do { _ = try resetTips() } catch { host?.problem(String(describing: error)) }
                host?.refreshTips()
            default: break
            }
        }

        private func open(_ link: String) {
            if let url = URL(string: link) { ExternalOpen.open(url) }
        }
    }
#endif

#if WITH_UNIFFI
    extension RustSettingsBackend: RefusalReporting {}
#endif
