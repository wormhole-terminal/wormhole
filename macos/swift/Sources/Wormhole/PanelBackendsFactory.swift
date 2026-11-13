// The backends the right panel reads through. Where the FFI is linked and the window is real (a program was asked for), git,
// worktrees, files, the project and the usage reports are the core's, over the selected terminal's folder; everywhere else
// (`--mock`, an unlinked build) they are the sample. Trust is enforced in Rust; owner authentication is Touch ID in a real
// run and WORMHOLE_AUTH's answer (cancel by default) in a test instance, so a check never raises the prompt.

import AuxModel
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

    #if WITH_UNIFFI
        private static var usage: RustUsageBackend?
    #endif

    /// `effects` performs what the project asks of the window; it is given the hub once that exists.
    static func make(_ options: Options, store: WorkspaceStore, policy: PlatformPolicy, effects: AnyObject?, profile: String) -> Made {
        var made = Made()
        var backends: PanelUI.PanelBackends {
            get { made.backends }
            set { made.backends = newValue }
        }
        #if WITH_UNIFFI
            guard options.isReal else { return made }
            // The viewer's colours are the core's (tree-sitter); a diff's lines arrive coloured.
            backends.syntax = CoreSyntaxHighlighter()
            let effects = effects as? PanelEffects
            if let effects {
                let authenticator: any Authenticating = policy.touchesSystem ? LocalAuthenticator() : ScriptedAuthenticator()
                do {
                    let files = try FilesBridge.makeBackends(authenticator: authenticator, effects: effects, profile: profile, usage: usage)
                    // The first window's is everyone's: a second window's tab moves with it, and it outlives that window.
                    usage = files.usage
                    (backends.files, backends.project, backends.usage) = (files.files, files.project, files.usage)
                    made.notes = files.notes
                    made.usage = files.usage.service
                } catch {
                    FileHandle.standardError.write(Data("panel: files, project and usage are the sample (\(error))\n".utf8)) // text: a line on stderr for whoever launched the app
                }
            }
            // Whether a worktree may be made, and what it runs first, is the core's (the project's trust); git only makes it.
            let project = backends.project
            let real = try? GitBridge.makeBackends(
                worktreeAllowed: { await project.worktreeRefusal(of: $0) },
                // A worktree was made: its folder gets a terminal, which first runs the trusted project's setup. Its ports
                // (`WORMHOLE_PORT`, `WORMHOLE_PORTS`) are in the shell's environment: the FFI's `spawn_options` reads them from the
                // worktrees Wormhole made, for any terminal started in or under the folder (as the GPUI spawn hook does).
                worktreeCreated: { made, _ in
                    let setup = await project.worktreeSetup(of: made.repo)
                    await MainActor.run { _ = store.send(.newTerminalIn(made.folder)) }
                    // Pasted (bracketed) and submitted into the terminal just made for it, as one line whatever it holds.
                    if let setup { await effects?.perform([.paste(setup, submit: true, in: .current)]) }
                })
            if let real {
                backends.git = real.git
                backends.worktrees = real.worktrees
            }
        #endif
        return made
    }
}
