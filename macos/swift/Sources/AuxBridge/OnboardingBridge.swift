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
    private var trustDesk: TrustDesk?
    /// The folder `trustDesk` reads, to make a new one only when the project the intro is about changes.
    private var trustRoot: String?
    private let frontProject: () -> URL?
    /// The last scan found no agent: the third step is the primer, and Start on it leaves the sidebar's note.
    private var primer = false
    private let onPrimer: () -> Void
    private let notesFeature: () -> Bool
    public var notesOn: Bool { notesFeature() }
    private let authenticator: any Authenticating
    private let recents: () -> [URL]
    private let dataDir: URL?
    private let windowPanel: (get: () -> Bool, set: (Bool) -> Void)
    private let onFinish: (_ folder: URL?, _ firstRun: Bool) -> Void
    private let hub = WakeHub()

    /// `preferences` start from the settings as they are; `project` is the folder the intro asks about, if it has
    /// a `.wormhole/` folder to trust. `onFinish` moves the first terminal to the folder picked.
    public init(
        preferences: OnboardingPreferences, settingsKeys: String, project: URL? = nil, recents: @escaping () -> [URL] = { [] },
        agentHome: URL? = nil, dataDir: URL? = nil, authenticator: any Authenticating,
        panel: (get: () -> Bool, set: (Bool) -> Void) = ({ false }, { _ in }),
        frontProject: @escaping () -> URL? = { nil }, onPrimer: @escaping () -> Void = {},
        notesOn: @escaping () -> Bool = { true },
        onFinish: @escaping (_ folder: URL?, _ firstRun: Bool) -> Void
    ) throws {
        run = try FirstRun(agentHome: agentHome?.path, dataDir: dataDir?.path)
        self.preferences = preferences
        self.settingsKeys = settingsKeys
        self.authenticator = authenticator
        self.recents = recents
        self.dataDir = dataDir
        windowPanel = panel
        self.onFinish = onFinish
        (self.frontProject, self.onPrimer, notesFeature) = (frontProject, onPrimer, notesOn)
        let wakes = hub
        trustDesk = try project.map { try TrustDesk(root: $0.path, profile: nil, dataDir: dataDir?.path, listener: wakes) }
        trustRoot = project?.path
        trustCard = trustDesk.flatMap(Self.card)
        preferences.onChange = { [weak self] in self?.writeLook() }
    }

    public func scanAgents() async -> (found: [FoundAgent], others: Int) {
        let run = run
        let scan = await Task.detached { run.scanAgents() }.value
        primer = scan.found.isEmpty
        return (scan.found.map(\.value), Int(scan.others))
    }

    public func installHooks(agent: String) async throws -> FoundAgent {
        let run = run
        return try await Task.detached { try run.installHooks(id: agent).value }.value
    }

    /// The card of the folder picked, else of the front terminal's project: its desk is made when that project changes, so the
    /// card follows the pick (and `trust` approves the project the card showed).
    public func trustCard(for folder: URL?) -> TrustCard? {
        guard let root = (folder ?? frontProject())?.path else { return trustCard }
        if root != trustRoot {
            trustRoot = root
            trustDesk = try? TrustDesk(root: root, profile: nil, dataDir: dataDir?.path, listener: hub)
            trustCard = trustDesk.flatMap(Self.card)
        }
        return trustCard
    }

    public func trust(_ folder: URL) {
        // Held before the dialog: the answer approves what was read now, not what is on disk afterwards.
        guard let trustDesk, trustDesk.review().problem == nil, trustDesk.holdReviewed() else { return }
        Task {
            do {
                try await authenticator.authenticate(reason: AuxText.trustTouchIDReason(folder: folder.lastPathComponent))
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
        panel.prompt = AuxText.startHere
        return await panel.begin() == .OK ? panel.url : nil
    }

    public func finish(folder: URL?, firstRun: Bool) {
        // A replay of the intro changes nothing.
        if firstRun {
            try? run.finish()
            if primer { onPrimer() }
        }
        onFinish(folder, firstRun)
    }

    /// The walk over core's, from the settings and the window's panel as they are.
    public func tailor() -> (any TailorWalking)? {
        try? FFITailor(panelOpen: windowPanel.get(), dataDir: dataDir)
    }

    /// An answer of the walk opens or closes the window's right panel.
    public func panelOpen(_ open: Bool) { windowPanel.set(open) }

    /// The newest terminal with something ticked by default (one with nothing to bring over leaves the theme file import).
    public func importOffer() async -> ImportChoice? {
        guard let offers = try? await ImportBridge.scan(), let found = offers.first(where: { $0.rows.contains(where: \.tickedByDefault) }) else {
            return nil
        }
        return ImportChoice(source: found.source, offer: importUseMineWords(terminal: found.name))
    }

    /// Brings over what the offered terminal's settings tick by default; says what came over.
    public func useImport(_ choice: ImportChoice) async throws -> String {
        let offers = try await ImportBridge.scan()
        let found = offers.first { $0.source == choice.source }
        let taken = try await ImportBridge.apply(source: choice.source, dataDir: dataDir)
        let left = max(0, (found?.rows.count ?? taken) - taken) + (found?.skipped ?? 0)
        return importBroughtWords(brought: UInt32(taken), left: UInt32(left))
    }

    /// Asks for a theme file and imports it as the theme.
    public func importThemeFile() async -> Bool {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.prompt = AuxText.importTheme
        guard await panel.begin() == .OK, let url = panel.url else { return false }
        return (try? WormholeBindings.importThemeFile(path: url.path, dataDir: dataDir?.path)) != nil
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
