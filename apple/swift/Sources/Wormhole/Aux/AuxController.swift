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
    func showTrust() { TrustWindow.show(backend: backends.trust(root: app.hub.panel.project.state?.root)) }

    /// The one updater the Updates window and the Settings page share.
    func updateBackend() -> any UpdateBackend {
        let backend = updates ?? backends.updates()
        updates = backend
        return backend
    }

    func showUpdates() { UpdateWindow.show(backend: updateBackend(), version: AppInfo.from(bundle: .main).version) }

    /// Agents' questions (the control API's approval) as sheets on the window, one at a time, as they arrive: profile
    /// `profile`'s (the shown one's by default), which the hook socket judges its windows' requests by from now on.
    func startAccess(profile: String? = nil) {
        let id = profile ?? app.windowsHost.profile
        guard access[id] == nil, let backend = backends.access(profile: id) else { return }
        accessBackends[id] = backend
        let driver = AccessDriver(backend: backend) { [weak app] in app?.window }
        access[id] = driver
        driver.start()
        #if WITH_UNIFFI
            if let desk = (backend as? FFIAccessBackend)?.desk { app.hooks?.addDesk(desk) }
        #endif
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
        let window = OnboardingWindow(backend: backends.onboarding(firstRun: firstRun), firstRun: firstRun, immersive: !app.policy.background)
        onboarding = window
        window.show()
    }

    /// Whether this data folder has not seen the welcome (and a test instance never shows it by itself).
    var isFirstRun: Bool { !app.policy.background && backends.shouldShowWelcome() }
}
