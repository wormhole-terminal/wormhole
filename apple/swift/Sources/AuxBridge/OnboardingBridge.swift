// `OnboardingBackend` over core's first-run pieces (`FirstRun`, `TrustDesk`): the folders the first step offers, the
// agents found on PATH and their hooks, the look the second step sets (written through core's settings as the
// person goes), the trust card of the project the intro is about, and the flag that the intro was seen. Every
// method is a request: nothing is installed, trusted or written until the button that asks for it is pressed.

import AppKit
import AuxModel
import Foundation
import PlatformModel
import WormholeBindings

@MainActor
public final class FFIOnboardingBackend: OnboardingBackend {
    public let preferences: OnboardingPreferences
    public let settingsKeys: String
    public private(set) var trustCard: TrustCard?

    private let run: FirstRun
    private let trustDesk: TrustDesk?
    private let authenticator: any Authenticating
    private let recents: () -> [URL]
    private let onFinish: (_ folder: URL?, _ firstRun: Bool) -> Void
    private let hub = WakeHub()

    /// `preferences` start from the settings as they are; `project` is the folder the intro asks about, if it has
    /// a `.wormhole/` folder to trust. `onFinish` moves the first terminal to the folder picked.
    public init(
        preferences: OnboardingPreferences, settingsKeys: String, project: URL? = nil, recents: @escaping () -> [URL] = { [] },
        agentHome: URL? = nil, dataDir: URL? = nil, authenticator: any Authenticating,
        onFinish: @escaping (_ folder: URL?, _ firstRun: Bool) -> Void
    ) throws {
        run = try FirstRun(agentHome: agentHome?.path, dataDir: dataDir?.path)
        self.preferences = preferences
        self.settingsKeys = settingsKeys
        self.authenticator = authenticator
        self.recents = recents
        self.onFinish = onFinish
        let wakes = hub
        trustDesk = try project.map { try TrustDesk(root: $0.path, profile: nil, dataDir: dataDir?.path, listener: wakes) }
        trustCard = trustDesk.flatMap(Self.card)
        preferences.onChange = { [weak self] in self?.writeLook() }
    }

    public func scanAgents() async -> (found: [FoundAgent], others: Int) {
        let run = run
        let scan = await Task.detached { run.scanAgents() }.value
        return (scan.found.map(\.value), Int(scan.others))
    }

    public func installHooks(agent: String) async throws -> FoundAgent {
        let run = run
        return try await Task.detached { try run.installHooks(id: agent).value }.value
    }

    public func trust(_ folder: URL) {
        guard let trustDesk, trustDesk.review().problem == nil else { return }
        Task {
            do {
                try await authenticator.authenticate(reason: "trust the project folder \u{201C}\(folder.lastPathComponent)\u{201D} and let it run commands")
                try trustDesk.approveReviewed()
            } catch {
                // Cancelled or refused: nothing is trusted.
            }
        }
    }

    public func places() -> [Place] {
        run.places(home: nil, recents: recents().map(\.path)).map {
            Place(kind: $0.kind.value, name: $0.name, url: URL(fileURLWithPath: $0.path), detail: $0.detail)
        }
    }

    public func chooseFolder() async -> URL? {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Start here"
        return await panel.begin() == .OK ? panel.url : nil
    }

    public func finish(folder: URL?, firstRun: Bool) {
        // A replay of the intro changes nothing.
        if firstRun { try? run.finish() }
        onFinish(folder, firstRun)
    }

    /// Whether the intro should open: a first launch nothing was saved from that never finished or skipped it.
    public func shouldShow(savedWindows: Int) -> Bool { run.shouldShow(savedWindows: UInt32(savedWindows)) }

    private func writeLook() {
        run.writeLook(
            look: LookRecord(
                primary: preferences.primary, fontFamily: preferences.fontFamily, fontSize: preferences.fontSize,
                ligatures: preferences.ligatures, notifications: preferences.notifications,
                notifyCommandDone: preferences.notifyCommandDone))
    }

    /// The card of a project that has something to approve; none when it is trusted, empty or does not load.
    private static func card(_ desk: TrustDesk) -> TrustCard? {
        let review = desk.review()
        guard review.status != .trusted, review.problem == nil, !review.lines.isEmpty else { return nil }
        return TrustCard(name: review.name, directory: review.directory, commands: review.lines.count)
    }
}

extension FoundAgentRecord {
    var value: FoundAgent {
        let hooks: FoundAgent.Hooks =
            switch self.hooks {
            case .notInstalled: .notInstalled
            case .partial: .partial
            case .installed: .installed
            case .unsupported: .unsupported
            }
        return FoundAgent(id: id, name: name, hooks: hooks, path: path)
    }
}

extension PlaceKindRecord {
    var value: Place.Kind {
        switch self {
        case .recent: .recent
        case .home: .home
        case .desktop: .desktop
        case .documents: .documents
        case .developer: .developer
        }
    }
}
