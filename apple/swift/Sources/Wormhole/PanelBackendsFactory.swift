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
        /// The core's usage service, which the hook socket feeds (a `UsageService`).
        var usage: AnyObject?
    }

    /// `effects` performs what the project asks of the window; it is given the hub once that exists.
    static func make(_ options: Options, store: WorkspaceStore, policy: PlatformPolicy, effects: AnyObject?, profile: String) -> Made {
        var made = Made()
        var backends: PanelUI.PanelBackends {
            get { made.backends }
            set { made.backends = newValue }
        }
        #if WITH_UNIFFI
            guard options.isReal else { return made }
            let effects = effects as? PanelEffects
            if let effects {
                let authenticator: any Authenticating = policy.touchesSystem ? LocalAuthenticator() : ScriptedAuthenticator()
                do {
                    let files = try FilesBridge.makeBackends(authenticator: authenticator, effects: effects, profile: profile)
                    (backends.files, backends.project, backends.usage) = (files.files, files.project, files.usage)
                    made.notes = files.notes
                    made.usage = files.usage.service
                } catch {
                    FileHandle.standardError.write(Data("panel: files, project and usage are the sample (\(error))\n".utf8))
                }
            }
            // Whether a worktree may be made, and what it runs first, is the core's (the project's trust); git only makes it.
            let project = backends.project
            let real = try? GitBridge.makeBackends(
                worktreeAllowed: { await project.worktreeRefusal(of: $0) },
                // A worktree was made: its folder gets a terminal, which first runs the trusted project's setup (its ports,
                // `environment`, are not passed on yet).
                worktreeCreated: { made, _ in
                    let setup = await project.worktreeSetup(of: made.repo)
                    await MainActor.run { _ = store.send(.newTerminalIn(made.folder)) }
                    if let setup { await effects?.perform([.type(setup, in: .current)]) }
                })
            if let real {
                backends.git = real.git
                backends.worktrees = real.worktrees
            }
        #endif
        return made
    }
}
