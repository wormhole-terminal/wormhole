// The Project tab's state: the colour of the terminal's folder, which works with or without a project folder, and
// the `.wormhole/` project at that folder with its trust. Everything that runs a command goes through the backend,
// where trust is checked; this only asks and shows what came back.

import Observation
import PanelModel
import WorkspaceModel

@MainActor @Observable
public final class ProjectStore {
    public private(set) var folder: String?
    public private(set) var band: String?
    public private(set) var state: ProjectState?
    public private(set) var paint = FolderPaint.own
    public private(set) var loaded = false
    public private(set) var error: String?
    /// What went wrong with the person's own stores, said where there is no project too.
    public private(set) var problem: String?
    /// The project's trust waits for Touch ID or the password.
    public private(set) var approving = false
    @ObservationIgnored private let backend: any ProjectBackend
    @ObservationIgnored private weak var host: (any PanelHost)?
    @ObservationIgnored private var watch: Task<Void, Never>?

    public init(backend: any ProjectBackend, host: (any PanelHost)? = nil) {
        self.backend = backend
        self.host = host
    }

    /// `band`: the band's folder (the repository's root, else the folder), which the colour and Create are for;
    /// the project is looked up from the folder itself.
    public func follow(_ folder: String?, band: String? = nil) {
        guard folder != self.folder || band != self.band else { return }
        (self.folder, self.band) = (folder, band)
        (state, loaded, error) = (nil, false, nil)
        watch?.cancel()
        guard let folder else { return }
        reload()
        watch = Task { [backend, weak self] in
            for await _ in backend.changes(at: folder) { self?.reload() }
        }
    }

    public func reload() {
        guard let folder else { return }
        let (backend, band) = (backend, band)
        Task {
            async let project = backend.project(at: folder)
            async let colour = backend.folderPaint(of: band ?? folder)
            async let stores = backend.problem()
            let (found, painted, wrong) = await (project, colour, stores)
            guard folder == self.folder, band == self.band else { return }
            (state, paint, loaded, problem) = (found, painted, true, wrong)
        }
    }

    public func setColor(_ color: FolderColor?) { act { folder, backend in await backend.setFolderColor(color, of: folder) } }
    public func setColor(hex: String) { act { folder, backend in await backend.setFolderColor(hex: hex, of: folder) } }

    public func create() {
        act { [weak self] folder, backend in
            let problem = await backend.createProject(at: folder)
            await MainActor.run { self?.error = problem }
        }
    }

    public func setFeature(_ feature: ProjectFeature, on: Bool) {
        guard let root = state?.root else { return }
        act { _, backend in await backend.setFeature(feature, on: on, at: root) }
    }

    public func approve() {
        guard let root = state?.root, !approving else { return }
        approving = true
        act { [weak self] _, backend in
            _ = await backend.approve(at: root)
            await MainActor.run { self?.approving = false }
        }
    }

    /// The approve button's words: Touch ID where it is set up, else the password.
    public var approveTitle: String {
        approving ? panelText("settings-projects-waiting-for-approval", "Waiting for approval\u{2026}")
            : backend.biometricsAvailable ? panelText("settings-projects-trust-with-touch-id", "Trust with Touch ID")
            : panelText("project-trust-password", "Trust (approve with password)")
    }

    /// "View all N…": the window of every line the project would run, with the approval there.
    public func viewAll() { host?.showProjectCommands() }

    public func revoke() {
        guard let root = state?.root else { return }
        act { _, backend in await backend.revoke(at: root) }
    }

    public func run(_ action: ProjectAction) {
        guard let root = state?.root else { return }
        act { _, backend in await backend.run(action: action.id, at: root) }
    }

    /// Runs the action whose key is `glyphs` when the core says, now, that the project is trusted and has one; false: the key is
    /// nobody's (nothing runs, and the window lets it through).
    public func runAction(forKey glyphs: String) -> Bool {
        guard let root = state?.root, let id = backend.actionID(forKey: glyphs, at: root) else { return false }
        act { _, backend in await backend.run(action: id, at: root) }
        return true
    }

    public func send(_ prompt: ProjectPrompt) {
        guard let root = state?.root else { return }
        act { _, backend in await backend.send(prompt: prompt.id, at: root) }
    }

    /// A prompt copied into a new note, to edit before it is sent.
    public func copyToNote(_ prompt: ProjectPrompt, in notes: NotesStore) {
        guard let root = state?.root else { return }
        let backend = backend
        Task {
            guard let text = await backend.text(ofPrompt: prompt.id, at: root) else { return }
            notes.add(note: text)
            host?.show(tab: .notes)
        }
    }

    public func openLayout() {
        guard let root = state?.root else { return }
        act { _, backend in await backend.openLayout(at: root) }
    }

    /// Runs a backend call for the current folder, then reads the tab again (the file watch would too, a beat later).
    private func act(_ call: @escaping @Sendable (String, any ProjectBackend) async -> Void) {
        guard let folder = band ?? folder else { return }
        let backend = backend
        Task {
            await call(folder, backend)
            reload()
        }
    }

    /// The project root of any terminal's folder (the notes' sync goes through every terminal, not only the shown one).
    public func root(of folder: String) async -> String? { await backend.project(at: folder)?.root }
}
