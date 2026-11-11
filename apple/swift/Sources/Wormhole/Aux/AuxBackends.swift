// The backends of the secondary windows: the core's where the window is real, the samples' everywhere else (`--mock`, an
// unlinked build). The updater never contacts a feed unless `--update-feed <url>` named one; without it the Updates window
// says so. Anything that can fail to start falls back to the sample and says why on stderr.

import AppKit
import AuxModel
import AuxWindows
import Foundation
import PlatformModel
import PlatformUI
import SettingsModel

#if WITH_UNIFFI
    import AuxBridge
    import WormholeBindings
#endif

@MainActor
struct AuxBackends {
    let app: AppController

    private var real: Bool { app.options.isReal }

    private var authenticator: any Authenticating {
        app.policy.touchesSystem ? LocalAuthenticator() : ScriptedAuthenticator()
    }

    /// The profile Wormhole is in: approvals and the project trust are its own, not the Default's (`nil`: the Default).
    private var activeProfileID: String? {
        #if WITH_UNIFFI
            try? activeProfile().id
        #else
            nil
        #endif
    }

    private func fallback(_ what: String, _ error: any Error) {
        FileHandle.standardError.write(Data("\(what): the core's is not available (\(error)); using the sample\n".utf8))
    }

    func hosts() -> any HostsBackend {
        #if WITH_UNIFFI
            if real {
                do {
                    let backend = try AuxBridge.makeHostsBackend()
                    // The window knows which terminals hold a host; the folder does not.
                    let attached = app.sessions.flatMap { session in
                        session.hub.state.sections.flatMap(\.rows).compactMap { row in
                            session.hub.store.hosting(of: row.id)?.hostID.map {
                                (hostID: $0, window: session.id, title: row.terminal.title)
                            }
                        }
                    }
                    backend.setAttachments(attached)
                    backend.setPersistentTerminals(app.hub.store.hosting(of: app.hub.state.selected ?? 0) != nil)
                    return backend
                } catch { fallback("session hosts", error) }
            }
        #endif
        return SampleHostsBackend()
    }

    /// The project the selected terminal is in, when the core found one and this is a real window.
    func trust(root: String?) -> any TrustBackend {
        #if WITH_UNIFFI
            if real, let root {
                do {
                    return try AuxBridge.makeTrustBackend(root: URL(fileURLWithPath: root), profile: activeProfileID, authenticator: authenticator)
                } catch { fallback("project trust", error) }
            }
        #endif
        return SampleTrustBackend()
    }

    func access() -> (any AccessBackend)? {
        #if WITH_UNIFFI
            if real {
                do { return try AuxBridge.makeAccessBackend(profile: activeProfileID) } catch { fallback("agent access", error) }
            }
        #endif
        return nil
    }

    func licenses() -> any LicenseSource {
        #if WITH_UNIFFI
            if real { return AuxBridge.makeLicenseSource() }
        #endif
        return BundledLicenseSource()
    }

    func updates() -> any UpdateBackend {
        #if WITH_UNIFFI
            if real, let feed = app.options.updateFeedURL, let key = publicKey() {
                let info = AppInfo.from(bundle: .main)
                let setup = FFIUpdateBackend.Setup(
                    baseURL: feed, publicKey: key, currentVersion: info.version, bundleID: Isolation.bundleIdentifier)
                let hooks = FFIUpdateBackend.Hooks(
                    runningTerminals: { [weak app] in app?.runningTerminals ?? [] }, terminate: { [weak app] in app?.terminateUnattended() })
                do { return try AuxBridge.makeUpdateBackend(setup: setup, hooks: hooks) } catch { fallback("updates", error) }
            }
        #endif
        return SampleUpdateBackend(phase: .unsupported("This build has no update feed. Start it with --update-feed <url> to check one."))
    }

    /// `packaging/updater.pub`: the minisign key releases are signed with.
    private func publicKey() -> String? {
        Options.findInRepository("packaging/updater.pub").flatMap { try? String(contentsOf: $0, encoding: .utf8) }
    }

    func onboarding(firstRun: Bool) -> any OnboardingBackend {
        #if WITH_UNIFFI
            if real {
                let values = app.settingsModel.backend.values
                let preferences = OnboardingPreferences(
                    fontFamily: values["font_family"]?.string ?? "JetBrains Mono", fontSize: values["font_size"]?.number ?? 14,
                    ligatures: values["font_ligatures"]?.bool ?? true)
                let home = ProcessInfo.processInfo.environment["WORMHOLE_AGENT_HOME"].map { URL(fileURLWithPath: $0) }
                do {
                    return try FFIOnboardingBackend(
                        preferences: preferences, settingsKeys: "\u{2318},", agentHome: home, dataDir: app.dataDirectory,
                        authenticator: authenticator
                    ) { [weak app] folder, _ in
                        if let folder { app?.hub.send(.newTerminalIn(folder.path)) }
                    }
                } catch { fallback("welcome", error) }
            }
        #endif
        return FirstRunBackend(marker: FirstRunBackend.marker(in: app.dataDirectory))
    }

    /// Whether the welcome has not been seen in this data folder: the core's flag when it is real, else the marker file.
    func shouldShowWelcome() -> Bool {
        #if WITH_UNIFFI
            if real, let backend = onboarding(firstRun: true) as? FFIOnboardingBackend { return backend.shouldShow(savedWindows: 0) }
        #endif
        return !FileManager.default.fileExists(atPath: FirstRunBackend.marker(in: app.dataDirectory).path)
    }
}
