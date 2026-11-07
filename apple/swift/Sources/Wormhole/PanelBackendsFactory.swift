// The backends the right panel reads through. Where the FFI is linked and the window is real (a program was asked for), git,
// worktrees, files, the project and the usage reports are the core's, over the selected terminal's folder; everywhere else
// (`--mock`, an unlinked build) they are the sample. Trust is enforced in Rust; owner authentication is Touch ID in a real
// run and WORMHOLE_AUTH's answer (cancel by default) in a test instance, so a check never raises the prompt.

import Foundation
import PanelModel
import PanelUI
import PlatformModel
import PlatformUI
import WorkspaceModel

#if WITH_UNIFFI
    import FilesBridge
    import GitBridge
#endif

@MainActor
enum PanelBackendsFactory {
    /// The backends, and where the Notes tab keeps its text (nil: nowhere, as in the sample).
    struct Made {
        var backends = PanelUI.PanelBackends.mock()
        var notes: (any NotesPersisting)?
    }

    /// `effects` performs what the project asks of the window; it is given the hub once that exists.
    static func make(_ options: Options, store: WorkspaceStore, policy: PlatformPolicy, effects: AnyObject?) -> Made {
        var made = Made()
        var backends: PanelUI.PanelBackends {
            get { made.backends }
            set { made.backends = newValue }
        }
        #if WITH_UNIFFI
            guard options.isReal else { return made }
            let project = backends.project
            let real = try? GitBridge.makeBackends(
                worktreeAllowed: { await refusal(making: $0, project: project) },
                // A worktree was made: its folder gets a terminal (its ports, `environment`, are not passed on yet).
                worktreeCreated: { made, _ in await MainActor.run { _ = store.send(.newTerminalIn(made.folder)) } })
            if let real {
                backends.git = real.git
                backends.worktrees = real.worktrees
            }
            if let effects = effects as? PanelEffects {
                let authenticator: any Authenticating = policy.touchesSystem ? LocalAuthenticator() : ScriptedAuthenticator()
                do {
                    let files = try FilesBridge.makeBackends(authenticator: authenticator, effects: effects)
                    (backends.files, backends.project, backends.usage) = (files.files, files.project, files.usage)
                    made.notes = files.notes
                } catch {
                    FileHandle.standardError.write(Data("panel: files, project and usage are the sample (\(error))\n".utf8))
                }
            }
        #endif
        return made
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
