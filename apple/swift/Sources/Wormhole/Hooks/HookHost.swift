// The app as the data folder's one running instance: the agents' hooks and the `Wormhole` command line reach the workspace
// through its socket (Rust serves it; `HookBridge`). What only the app can do arrives here as actions: come forward, show
// a notification, ask whether to close a terminal. A second launch of the same data folder is asked to come forward by the
// first and quits.

#if WITH_UNIFFI
    import AppKit
    import AuxBridge
    import FilesBridge
    import Foundation
    import HookBridge
    import PanesModel
    import WorkspaceBridge
    import WormholeBindings

    @MainActor
    final class HookHost {
        private let handle: HookServerHandle
        private var listening: Task<Void, Never>?
        private unowned let app: AppController

        /// Nil when this window is not the core's, or when the parts the server shares are not there.
        init?(app: AppController) throws {
            guard app.options.isReal, let workspace = (app.hub.store.hooks as? RustWorkspaceBackend)?.core,
                let usage = app.panelUsage as? UsageService, let access = (app.aux.accessBackend as? FFIAccessBackend)?.desk
            else { return nil }
            self.app = app
            handle = try HookBridge.start(workspace: workspace, usage: usage, access: access)
        }

        func start() {
            let actions = handle.actions
            listening = Task { [weak self] in
                for await batch in actions { batch.forEach { self?.perform($0) } }
            }
        }

        func stop() { handle.stop() }

        /// Settings → Agent access → Revoke: the approval is taken back and the streams it let in end.
        func revokeApproval(_ number: UInt64, stored: Bool) {
            if stored { handle.revokeStoredApproval(number) } else { handle.revokeApproval(number) }
        }

        /// Settings changed the agent-access mode: the server reads it again.
        func settingsChanged() { handle.settingsChanged() }

        private func perform(_ action: HookAction) {
            switch action {
            case .activate: if app.policy.touchesSystem { NSApp.activate() }
            case .notice(let terminal, let title, let body): app.notices?.post(terminal: terminal, title: title, body: body)
            case .closeAsked(let terminal):
                app.closeTerminal(Int(terminal)) { [weak self] in self?.handle.closeQuestionOver() }
            case .worktreeAsked(let folder, let branch): app.hub.askWorktree(branch: branch, in: folder)
            case .gauge(let terminal, let agent, let model, let context):
                app.hub.gauges.tell(terminal: Int(terminal), agent: agent, gauge: GaugeFact(model: model, context: context))
            }
        }
    }

    extension AppController {
        /// Serves the data folder's socket; a second launch of the same folder quits (the first was asked to come forward).
        func startHooks() {
            do {
                hooks = try HookHost(app: self)
                hooks?.start()
            } catch HookError.AlreadyServed {
                FileHandle.standardError.write(Data("another Wormhole serves this data folder: it was asked to come forward\n".utf8))
                terminateUnattended()
            } catch {
                FileHandle.standardError.write(Data("hooks: the socket is not served (\(error)); agents' hooks will not arrive\n".utf8))
            }
        }
    }
#else
    /// Without the core there is no socket to serve.
    @MainActor
    final class HookHost {
        func settingsChanged() {}
        func revokeApproval(_ number: UInt64, stored: Bool) {}
        func stop() {}
    }

    extension AppController {
        func startHooks() {}
    }
#endif
