// The secondary windows, opened from the menus, the status item and the control folder: About, Licenses,
// Updates, Session Hosts, the project commands' trust window, the agent access sheet and the welcome. Each is
// over a sample backend until the core supplies the real one through the FFI; the protocol seams are the
// AuxModel ones, so only the backend changes then.

import AppKit
import AuxModel
import AuxWindows

@MainActor
final class AuxController {
    private let dataDirectory: URL
    private let updates = SampleUpdateBackend(phase: .upToDate)
    private var onboarding: OnboardingWindow?
    private unowned let app: AppController

    init(app: AppController, dataDirectory: URL) {
        self.app = app
        self.dataDirectory = dataDirectory
    }

    func showAbout() { AboutPanel.show() }
    func showLicenses() { Licenses.show(source: BundledLicenseSource()) }
    func showHosts() { HostsWindow.show(backend: SampleHostsBackend()) }
    func showTrust() { TrustWindow.show(backend: SampleTrustBackend()) }

    func showUpdates() {
        UpdateWindow.show(backend: updates, version: AppInfo.from(bundle: .main).version)
    }

    /// The control API's approval question on the window, as a sheet; the answer is dropped (nothing asks yet).
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
        let backend = FirstRunBackend(marker: FirstRunBackend.marker(in: dataDirectory))
        let window = OnboardingWindow(backend: backend, firstRun: firstRun, immersive: !app.policy.background)
        onboarding = window
        window.show()
    }

    /// Whether this data folder has not seen the welcome (and a test instance never shows it by itself).
    var isFirstRun: Bool { !app.policy.background && !FileManager.default.fileExists(atPath: FirstRunBackend.marker(in: dataDirectory).path) }
}
