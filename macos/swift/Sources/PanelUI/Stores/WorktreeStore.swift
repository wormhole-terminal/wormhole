// A worktree per terminal, as the panel hosts it: the sheet that asks for a branch before making one, and the alerts that say
// why one cannot be made, was not made or was not removed. Removing is the close question's (`AppController+WorktreeClose`).
// Writes only on the person's action, and the backend refuses in a project that isn't trusted.

import Foundation
import Observation
import PanelModel

@MainActor @Observable
public final class WorktreeStore {
    public private(set) var folder: String?
    /// The new-worktree sheet is up.
    public var asking = false {
        didSet {
            if !asking { (askedFolder, askedCommand) = (nil, nil) }
            if asking != oldValue { sheetChanged(asking) }
        }
    }
    /// The sheet came up or went away: the window tells the control line, which answers a second `--worktree` request Failed
    /// while one is up.
    @ObservationIgnored public var sheetChanged: (Bool) -> Void = { _ in }
    /// What `--command` asked to run in the terminal that opens, after the project's setup.
    @ObservationIgnored private var askedCommand: String?
    /// The repository a command-line request named, for as long as its sheet is up; else the terminal's folder.
    @ObservationIgnored private var askedFolder: String?
    public var branch = "" { didSet { if branch != oldValue { replan() } } }
    public private(set) var plan: WorktreePlan?
    public private(set) var error: String?
    public private(set) var busy = false
    /// An alert to show: a worktree that can't be made here, that git refused to make, or to remove.
    public var alert: Alert?
    /// `wh new-terminal --worktree <branch> --command <cmd>`: typed into the terminal that opens once Create is pressed (the
    /// panel's `create` takes it with it); nil for a sheet the person opened.
    public var firstCommand: String?

    public struct Alert: Identifiable, Equatable {
        public let id = UUID()
        public var title: String
        public var message: String
    }
    @ObservationIgnored private let backend: any WorktreeBackend
    @ObservationIgnored private weak var host: (any PanelHost)?
    @ObservationIgnored private var planning: Task<Void, Never>?

    public init(backend: any WorktreeBackend, host: (any PanelHost)? = nil) {
        self.backend = backend
        self.host = host
    }

    public func follow(_ folder: String?) {
        self.folder = folder
    }

    /// Settings → Terminal: where worktrees go and whether build folders are cloned into them.
    public func setOptions(base: String?, cloneBuildFolders: Bool) {
        backend.setOptions(base: base, cloneBuildFolders: cloneBuildFolders)
        if asking { replan() }
    }

    /// The worktree Wormhole made that `folder` is in, if any (the close question asks).
    public func made(at folder: String) async -> MadeWorktree? { await backend.made(at: folder) }

    /// Removes a worktree the close question offered to remove, once its terminal is closed; git's refusal (changes made in the
    /// meantime) is an alert.
    public func remove(_ worktree: MadeWorktree) async {
        if let failure = await backend.remove(worktree) { alert = Alert(title: panelText("worktree-remove-failed", "Couldn't remove the worktree"), message: failure) }
    }

    /// The sheet's footnote under the branch field: the folder it would make, or why it can't.
    public var footnote: String? {
        switch plan {
        case let .ready(_, folder): panelText("worktree-folder", ["folder": .text(shown(folder))], "Folder: \(shown(folder))")
        case let .refused(why): why
        case nil: panelText("worktree-folder-pending", "Folder: <branch>")
        }
    }

    /// Create stays pressable while a branch is typed: a refused one answers with its reason, as the dialog does.
    public var canCreate: Bool { !branch.isEmpty && !busy }

    /// The repository's name, which the sheet's description names.
    public var repositoryName: String { repository.map { ($0 as NSString).lastPathComponent } ?? panelText("worktree-this-repository", "the repository") }

    /// A path with the home folder as `~`.
    private func shown(_ path: String) -> String {
        guard let home = host?.homeFolder, path == home || path.hasPrefix(home + "/") else { return path }
        return "~" + path.dropFirst(home.count)
    }

    /// The sheet, empty or with `branch` filled in, for the terminal's folder or for `repo` (`wh new-terminal
    /// --worktree <branch>` names the folder it was run for). The backend may refuse (an untrusted project): that is said, not asked.
    public func ask(branch preset: String = "", in repo: String? = nil, command: String? = nil) {
        guard let target = repo ?? folder else { return }
        // A sheet already up is not reset by a second request (the control line answered that one Failed).
        guard !asking else { return }
        // Before the branch, which plans for the folder it finds.
        askedFolder = repo
        askedCommand = command
        plan = nil
        error = nil
        branch = ""
        branch = preset
        Task {
            if let problem = await backend.problem(making: target) {
                (askedFolder, askedCommand) = (nil, nil)
                sheetChanged(false)
                alert = Alert(title: panelText("worktree-refused-title", "Can\u{2019}t make a worktree here"), message: problem)
            } else {
                asking = true
            }
        }
    }

    var repository: String? { askedFolder ?? folder }

    private func replan() {
        guard let folder = repository else { return }
        planning?.cancel()
        error = nil
        guard !branch.isEmpty else { return plan = nil }
        planning = Task {
            let next = await backend.plan(branch: branch, repo: folder)
            if !Task.isCancelled { plan = next }
        }
    }

    public func create() {
        guard let folder = repository, canCreate else { return }
        // A refused branch says why under the field; nothing is asked of git.
        if case .refused = plan { return }
        busy = true
        let (typed, command) = (branch, askedCommand)
        Task {
            // Trust may have been revoked while the sheet was up: asked again, and a refusal keeps the sheet up with its reason.
            if let why = await backend.problem(making: folder) {
                busy = false
                return error = why
            }
            let failure = await backend.create(branch: typed, repo: folder, command: command)
            busy = false
            asking = false
            if let failure { alert = Alert(title: panelText("worktree-create-failed", "Couldn\u{2019}t create the worktree"), message: failure) }
        }
    }
}
