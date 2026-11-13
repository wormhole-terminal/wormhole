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
import WorkspaceBridge
import WorkspaceModel

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

    private func fallback(_ what: String, _ error: any Error) {
        FileHandle.standardError.write(Data("\(what): the core's is not available (\(error)); using the sample\n".utf8)) // text: a line on stderr for whoever launched the app
    }

    func hosts() -> any HostsBackend {
        #if WITH_UNIFFI
            if real {
                do {
                    // The windows know which terminals hold a host and how each connection stands; the folder does not. Read at each
                    // look, so a terminal attached or ended later shows the right standing.
                    return try AuxBridge.makeHostsBackend { [weak app] in
                        guard let app else { return ([], false) }
                        return (app.hostAttachments(), app.profileFlags.on(profile: app.windowsHost.profile).contains("persistent_terminals"))
                    }
                } catch { fallback("session hosts", error) } // text: names the part in a stderr line
            }
        #endif
        return SampleHostsBackend()
    }

    /// The project the selected terminal is in, when the core found one and this is a real window.
    /// `profile`: the window's own, whose approvals the window's terminals run under (not the one Wormhole is in).
    func trust(root: String?, profile: String) -> any TrustBackend {
        #if WITH_UNIFFI
            if real, let root {
                do {
                    return try AuxBridge.makeTrustBackend(root: URL(fileURLWithPath: root), profile: profile, authenticator: authenticator)
                } catch { fallback("project trust", error) } // text: names the part in a stderr line
            }
            // A real window with no project: it says so, rather than showing a sample's commands.
            if real, root == nil { return ClosedProjectTrust() }
        #endif
        return SampleTrustBackend()
    }

    func access(profile: String) -> (any AccessBackend)? {
        #if WITH_UNIFFI
            if real {
                do { return try AuxBridge.makeAccessBackend(profile: profile, resolver: accessResolver()) } catch { fallback("agent access", error) } // text: names the part in a stderr line
            }
        #endif
        return nil
    }

    #if WITH_UNIFFI
        /// What only the windows know for the agent access sheet: a terminal's row title and folder (any window's), and the
        /// terminal a program runs in, by the core's walk up the process tree (`sheet.rs` `terminal_display`, `terminal_of`).
        private func accessResolver() -> FFIAccessBackend.Resolver {
            FFIAccessBackend.Resolver(
                terminal: { [weak app] id in
                    app?.sessions.lazy.compactMap { $0.hub.state.all.first { Int64($0.id) == id } }.first
                        .map { .init(name: $0.title, folder: $0.folder) }
                },
                terminalOf: { [weak app] pid in
                    guard let app else { return nil }
                    for session in app.sessions {
                        guard let core = (session.hub.store.hooks as? RustWorkspaceBackend)?.core, let id = core.terminalRunning(pid: pid)
                        else { continue }
                        if let found = session.hub.state.all.first(where: { Int64($0.id) == id }) { return found.title }
                    }
                    return nil
                })
        }
    #endif

    func licenses() -> any LicenseSource {
        #if WITH_UNIFFI
            if real { return AuxBridge.makeLicenseSource() }
        #endif
        return BundledLicenseSource()
    }

    #if WITH_UNIFFI
        /// Where and what to update. A fixture names its own feed and key (`--update-feed`, `--update-key`, `--update-bundle`: they
        /// are what the tests use); a release build has its feed, key, team and fallback baked in when it was built, the way the GPUI app
        /// has them (`update::feed_config`), and its channel is Settings'. A test instance never takes the baked feed: it would ask a real
        /// server (and be counted); a dev build has none.
        func updateSetup() -> FFIUpdateBackend.Setup? {
            guard real else { return nil }
            let info = AppInfo.from(bundle: .main)
            if let feed = app.options.updateFeedURL, let key = publicKey() {
                var setup = FFIUpdateBackend.Setup(baseURL: feed, publicKey: key, currentVersion: info.version, bundleID: Isolation.bundleIdentifier)
                if app.options.updateKeyFile != nil || app.options.updateBundle != nil {
                    // A fixture: its own key and a bundle in the data folder, which no one has code-signed.
                    setup.appBundle = scratchBundle()
                    setup.verifyCodesign = false
                }
                return setup
            }
            guard app.policy.touchesSystem, let release = updateReleaseSetup() else { return nil }
            var setup = FFIUpdateBackend.Setup(baseURL: release.baseUrl, publicKey: release.publicKey, currentVersion: info.version, bundleID: Isolation.bundleIdentifier)
            setup.teamID = release.teamId
            setup.fallbackURL = release.fallbackUrl
            if Bundle.main.bundleURL.pathExtension == "app" { setup.appBundle = Bundle.main.bundleURL }
            setup.channel = app.settingsModel.backend.values["update_channel"]?.string == "unstable" ? .preview : .stable
            return setup
        }
    #endif

    func updates() -> any UpdateBackend {
        #if WITH_UNIFFI
            if let setup = updateSetup() {
                let hooks = FFIUpdateBackend.Hooks(
                    runningTerminals: { [weak app] in app?.runningTerminals ?? [] }, terminate: { [weak app] in app?.terminateUnattended() },
                    whenIdle: { [weak app] restart in app?.updateFollower?.restartWhenIdle(restart) ?? restart() },
                    cancelWhenIdle: { [weak app] in app?.updateFollower?.cancelRestartWhenIdle() })
                do { return try AuxBridge.makeUpdateBackend(setup: setup, hooks: hooks) } catch { fallback("updates", error) }
            }
        #endif
        return SampleUpdateBackend(phase: .unsupported(AuxText.noUpdateFeed))
    }

    /// The scratch bundle an install replaces: only one inside the data folder, never an installed app.
    private func scratchBundle() -> URL? {
        guard let bundle = app.options.updateBundle?.standardizedFileURL else { return nil }
        guard bundle.path.hasPrefix(app.dataDirectory.standardizedFileURL.path + "/") else {
            fallback("updates", Isolation.Refusal(description: "--update-bundle must be inside the data folder")) // text: a launch flag's error on stderr
            return nil
        }
        return bundle
    }

    /// `packaging/updater.pub`: the minisign key releases are signed with; a fixture names its own.
    private func publicKey() -> String? {
        if let file = app.options.updateKeyFile { return try? String(contentsOf: file, encoding: .utf8) }
        return Options.findInRepository("packaging/updater.pub").flatMap { try? String(contentsOf: $0, encoding: .utf8) }
    }

    func onboarding(firstRun: Bool) -> any OnboardingBackend {
        #if WITH_UNIFFI
            if real {
                let values = app.settingsModel.backend.values
                let preferences = OnboardingPreferences(
                    fontFamily: values["font_family"]?.string ?? "JetBrains Mono", fontSize: values["font_size"]?.number ?? 14, // text: a font family name
                    ligatures: values["font_ligatures"]?.bool ?? true)
                let home = ProcessInfo.processInfo.environment["WORMHOLE_AGENT_HOME"].map { URL(fileURLWithPath: $0) } // text: an environment variable
                do {
                    return try FFIOnboardingBackend(
                        preferences: preferences, settingsKeys: "\u{2318},",
                        recents: { [weak app] in (app.map { $0.recents.roots(of: $0.front.windows.profile) } ?? []).map { URL(fileURLWithPath: $0) } },
                        agentHome: home, dataDir: app.dataDirectory, authenticator: authenticator,
                        // "How much to show" opens or closes the right panel of the window behind (the window's own, so not a setting).
                        panel: (
                            { [weak app] in app?.hub.state.panelVisible ?? false },
                            { [weak app] open in if let app, app.hub.state.panelVisible != open { app.hub.send(.togglePanel) } }
                        ),
                        // The project the trust card is about when no folder was picked: the front terminal's.
                        frontProject: { [weak app] in app?.hub.panel.project.state.map { URL(fileURLWithPath: $0.root) } },
                        // The primer's Start leaves the sidebar's "no agents" note, unless it was closed for good.
                        onPrimer: { [weak app] in
                            if !((try? noAgentsTipDismissed()) ?? false) { app?.hub.agentsTip = .note }
                        },
                        // Notes on in the window's own profile: the primer's words and its "Jot a note" line follow it.
                        notesOn: { [weak app] in app.map { !$0.featuresOff($0.front).contains("notes") } ?? true },
                        // The project is trusted as the window's own profile, which is not always the shown one.
                        profile: { [weak app] in app?.front.windows.profile }
                    ) { [weak app] folder, firstRun in
                        // The untouched first terminal gives way to one in the folder, only while it is the window's only one and is
                        // not there already (`window/onboarding_host.rs`).
                        guard firstRun, let app, let folder else { return }
                        let hub = app.hub
                        guard hub.state.all.count == 1, let first = hub.state.all.first else { return }
                        let there = first.folder.map { URL(fileURLWithPath: ($0 as NSString).expandingTildeInPath).standardizedFileURL }
                        guard there != folder.standardizedFileURL else { return }
                        hub.send(.newTerminalIn(folder.path))
                        app.closeTerminal(first.id)
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

#if WITH_UNIFFI
    extension AppController {
        /// The terminals of every window that hold a session host, with how their connection stands and, where a host that runs
        /// cannot be shown, why.
        func hostAttachments() -> [HostAttachmentRecord] {
            sessions.flatMap { session in
                let core = session.hub.store.hooks as? RustWorkspaceBackend
                // Every terminal of the window, those in folded groups or filtered out of the list too (`host_attachments`).
                return session.hub.state.all.compactMap { terminal -> HostAttachmentRecord? in
                    guard let hostID = session.hub.store.hosting(of: terminal.id)?.hostID else { return nil }
                    var why: HostUnavailableKind?
                    if case .unavailable(let kind)? = session.hub.store.hostProblem(of: terminal.id) { why = kind.record }
                    return HostAttachmentRecord(
                        hostId: hostID, window: Int64(session.id), title: terminal.title, link: core?.hostLink(of: terminal.id),
                        unavailable: why, unavailableDetail: core?.unavailableDetail(of: terminal.id))
                }
            }
        }
    }

    extension HostProblem.Why {
        var record: HostUnavailableKind {
            switch self {
            case .refused: .refused
            case .tooOld: .tooOld
            case .tooNew: .tooNew
            case .busy: .busy
            case .timedOut: .timedOut
            case .unreachable: .unreachable
            case .failed: .failed
            }
        }
    }
#endif
