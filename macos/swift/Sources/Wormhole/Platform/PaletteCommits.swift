// The commits the palette lists: the log of HEAD of the selected terminal's folder, read through the core once each time the
// palette opens, off the main thread (GPUI's `load_commits`), and shown when they arrive.

import Observation
import PaletteModel
import PanelModel

@MainActor @Observable
final class PaletteCommits {
    private(set) var commits: [PaletteCommit] = []
    @ObservationIgnored private var loading: Task<Void, Never>?

    /// Reads the log again; no backend or folder (History off, no terminal in a repository) leaves the list empty.
    func load(from git: (any GitBackend)?, folder: String?) {
        loading?.cancel()
        commits = []
        guard let git, let folder else { return }
        loading = Task { [weak self] in
            guard let log = try? await git.log(in: folder, of: nil, paths: []), !Task.isCancelled else { return }
            self?.commits = log.commits.map { PaletteCommit(sha: $0.sha, short: $0.short, subject: $0.subject, author: $0.author) }
        }
    }
}
