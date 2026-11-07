// The backends the right panel reads through. Where the FFI is linked and the window is real (a program was asked for), git
// and worktrees are the core's, over the selected terminal's folder; everywhere else (`--mock`, an unlinked build, an
// instance that is not isolated) they are the sample. Files, project and usage are still the sample.

import PanelModel
import PanelUI
import WorkspaceModel

#if WITH_UNIFFI
    import GitBridge
#endif

@MainActor
enum PanelBackendsFactory {
    static func make(_ options: Options, store: WorkspaceStore) -> PanelBackends {
        var backends = PanelBackends.mock()
        #if WITH_UNIFFI
            guard options.program != nil else { return backends }
            let project = backends.project
            let real = try? GitBridge.makeBackends(
                worktreeAllowed: { await refusal(making: $0, project: project) },
                // A worktree was made: its folder gets a terminal (its ports, `environment`, are not passed on yet).
                worktreeCreated: { made, _ in await MainActor.run { _ = store.send(.newTerminalIn(made.folder)) } })
            if let real {
                backends.git = real.git
                backends.worktrees = real.worktrees
            }
        #endif
        return backends
    }

    /// Why a worktree may not be made of `folder`, or nil. Git knows nothing of trust, so the project's own state decides:
    /// only a trusted project is written to, and a folder whose project is unknown is not trusted.
    nonisolated static func refusal(making folder: String, project: any ProjectBackend) async -> String? {
        guard let state = await project.project(at: folder), state.status == .trusted else {
            return "Worktrees are made only in a trusted project: approve this folder's project in the Project tab first."
        }
        return nil
    }
}
