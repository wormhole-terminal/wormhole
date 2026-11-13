// The secondary windows, opened from the menus, the status item and the control folder: About, Licenses, Updates, Session
// Hosts, the project commands' trust window, the agent access sheet and the welcome. Their backends are `AuxBackends`'s
// (the core's where the window is real); the windows are AuxWindows'.

import AppKit
import AuxModel
import AuxWindows
#if WITH_UNIFFI
    import AuxBridge
#endif

@MainActor
final class AuxController {
    private let dataDirectory: URL
    private var updates: (any UpdateBackend)?
    var onboarding: OnboardingWindow?
    /// The run Play Intro and Play Welcome start (one plays at a time).
    let introPlayer = IntroPlayer()
    /// Each profile's agent-access questions, as sheets (a profile is its own trust domain: its desk, its approvals).
    private var access: [String: AccessDriver] = [:]
    private var accessBackends: [String: any AccessBackend] = [:]
    /// The shown profile's agent-access desk, shared with the hook socket; nil in the sample.
    var accessBackend: (any AccessBackend)? { accessBackends[app.windowsHost.profile] }
    unowned let app: AppController
    var backends: AuxBackends { AuxBackends(app: app) }

    init(app: AppController, dataDirectory: URL) {
        self.app = app
        self.dataDirectory = dataDirectory
    }

    func showAbout() { AboutPanel.show() }
    func showLicenses() { Licenses.show(source: backends.licenses()) }
    func showHosts() { HostsWindow.show(backend: backends.hosts()) }

    /// The project of the terminal in front, when the core found one.
    func showTrust() {
        let root = app.hub.panel.project.state?.root
        let backend = backends.trust(root: root, profile: app.front.windows.profile)
        TrustWindow.show(backend: ReloadingTrust(backend) { [weak app] in app?.hub.panel.project.reload() }, root: root)
    }

    /// The one updater the Updates window and the Settings page share.
    func updateBackend() -> any UpdateBackend {
        let backend = updates ?? backends.updates()
        updates = backend
        return backend
    }

    /// Whether this run has an updater (a feed from the build or a fixture's), so the schedule and the chip follow it.
    var hasUpdater: Bool {
        #if WITH_UNIFFI
            backends.updateSetup() != nil
        #else
            false
        #endif
    }

    /// Check for Updates…: the window, and a manual check started at once when nothing is in progress (`app/handlers.rs`).
    func showUpdates() {
        let backend = updateBackend()
        UpdateWindow.show(backend: backend, version: AppInfo.from(bundle: .main).version)
        if backend.phase == .idle || backend.phase == .upToDate { Task { await backend.check() } }
    }

    /// Agents' questions (the control API's approval) as sheets on the window, one at a time, as they arrive: profile
    /// `profile`'s (the shown one's by default), which the hook socket judges its windows' requests by from now on.
    func startAccess(profile: String? = nil) {
        let id = profile ?? app.windowsHost.profile
        guard access[id] == nil, let backend = backends.access(profile: id) else { return }
        accessBackends[id] = backend
        let driver = AccessDriver(backend: backend) { [weak self] in self?.window(ofProfile: id) }
        access[id] = driver
        driver.start()
        #if WITH_UNIFFI
            if let desk = (backend as? FFIAccessBackend)?.desk { app.hooks?.addDesk(desk) }
        #endif
    }

    /// The shown profile's question driver, for the control folder's `agent-ask`.
    var accessDriver: AccessDriver? { access[app.windowsHost.profile] }

    /// The window a question of `profile` goes on: the one in front if it is of that profile, else any shown one of it
    /// (`agent_access/ask.rs` `window_for`); nil when none is open.
    private func window(ofProfile profile: String) -> NSWindow? {
        let own = app.shown.filter { $0.windows.profile == profile }
        return (own.first { $0.window?.isKeyWindow == true } ?? own.first { $0 === app.lastFront } ?? own.first)?.window
    }

    /// A sample of the question, for the control folder's `access`: nothing real asks until the control socket serves one.
    func showAccess() {
        let request = AccessRequest(
            label: "claude", capability: .run, kind: .developerID,
            program: .init(path: "/usr/local/bin/claude", pid: 4242, signer: .init(kind: .developerID, subject: "Anthropic PBC")))
        let sheet = AccessSheet(request)
        Task { _ = await sheet.ask(in: app.window) }
    }

    /// The first run shows the welcome by itself, once per data folder; `--onboarding` and Help ▸ Welcome show it
    /// again. A test instance keeps the effect above the window instead of taking the screen.
    func showOnboarding(firstRun: Bool) {
        // Help ▸ Welcome while the page is up plays it again (`onboarding_host.rs`).
        if let open = onboarding, open.isOpen, !firstRun {
            open.replay()
            return
        }
        let window = OnboardingWindow(backend: backends.onboarding(firstRun: firstRun), firstRun: firstRun, immersive: !app.policy.background)
        onboarding = window
        window.show()
    }

    /// Whether this data folder has not seen the welcome (and a test instance never shows it by itself).
    var isFirstRun: Bool { !app.policy.background && backends.shouldShowWelcome() }
}
