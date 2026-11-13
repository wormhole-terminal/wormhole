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
    import WindowUI
    import WorkspaceBridge
    import WormholeBindings

    @MainActor
    final class HookHost {
        let handle: HookServerHandle
        private var listening: Task<Void, Never>?
        /// Tells the core which window is in front (`start`).
        private var frontObserver: NSObjectProtocol?
        /// The Probes are read again about once a minute, the backstop for what changes behind the app's back.
        private var probeBackstop: Task<Void, Never>?
        /// The task that tells the welcome what macOS says of banners (`HookHost+Welcome`).
        var permissionWatch: Task<Void, Never>?
        unowned let app: AppController

        /// Nil when this window is not the core's, or when the parts the server shares are not there.
        init?(app: AppController) throws {
            guard app.options.isReal, let workspace = (app.sessions.first?.hub.store.hooks as? RustWorkspaceBackend)?.core,
                let usage = app.panelUsage as? UsageService, let access = (app.aux.accessBackend as? FFIAccessBackend)?.desk
            else { return nil }
            self.app = app
            handle = try HookBridge.start(workspace: workspace, usage: usage, access: access)
        }

        func start() {
            // The window in front is what a control request without a window acts on (the core cannot see AppKit's key window).
            frontObserver = NotificationCenter.default.addObserver(forName: NSWindow.didBecomeKeyNotification, object: nil, queue: .main) { [weak self] note in
                MainActor.assumeIsolated {
                    guard let self, let window = note.object as? NSWindow, let session = self.app.sessions.first(where: { $0.window === window }) else { return }
                    self.handle.setFrontWindow(Int64(session.id))
                }
            }
            let actions = handle.actions
            listening = Task { [weak self] in
                for await batch in actions { batch.forEach { self?.perform($0) } }
            }
            probeBackstop = Task { [weak self] in
                while !Task.isCancelled {
                    // one-shot: the minute after which the Probes are read again (the backstop of a watch that missed an edit)
                    try? await Task.sleep(for: .seconds(60))
                    self?.handle.reloadProbes()
                }
            }
        }

        func stop() {
            probeBackstop?.cancel()
            handle.stop()
        }

        /// macOS asked to open these files and folders: a terminal each in window `window` (`SystemOpen.swift`).
        @discardableResult
        func openFromSystem(_ urls: [URL], in window: Int) -> Int { Int(handle.openFromSystem(urls: urls.map(\.absoluteString), window: Int64(window))) }

        /// Settings → Agent access → Revoke: the approval of `profile`'s desk is taken back and the streams it let in end.
        func revokeApproval(_ number: UInt64, stored: Bool, profile: String) {
            if stored { handle.revokeStoredApproval(profile: profile, number) } else { handle.revokeApproval(profile: profile, number) }
        }

        /// Settings → Agent access → a tick of an approved program's row.
        func setApprovalCapability(_ number: UInt64, stored: Bool, capability: AccessCapability, on: Bool, profile: String) {
            handle.setApprovalCapability(profile: profile, number, stored: stored, capability: capability, on: on)
        }

        /// A profile opened beside the first one: its windows' requests are judged by its own desk.
        func addDesk(_ desk: AccessDesk) { handle.addDesk(desk) }

        /// Settings changed profile `profile`'s agent-access mode: the server reads it again.
        func settingsChanged(profile: String) { handle.settingsChanged(profile: profile) }

        /// What the Probes set changed: the status bar says it, and every terminal takes a frame (rules, tints and folds are
        /// in the frames).
        private func probesChanged() {
            app.look.probeItems = handle.probeStatusItems().map {
                ProbeStatusItem(key: "\($0.probe)/\($0.item)", text: $0.text, token: $0.token, tooltip: $0.tooltip)
            }
            app.sessions.forEach { $0.hub.terminals.forEach { $0.requestRedraw() } }
        }

        private func perform(_ action: HookAction) {
            switch action {
            case .activate: app.activateFromSecondLaunch()
            case .front(let terminal): app.show(terminal: Int(terminal))
            case .closeAsked(let terminal):
                app.closeTerminal(Int(terminal), fromCommandLine: true) { [weak self] in self?.handle.closeQuestionOver() }
            case .worktreeAsked(let folder, let branch, let command):
                // The sheet tells the control line when it is gone, so a second request is answered Failed while it is up.
                app.hub.panel.worktrees.sheetChanged = { [weak self] up in self?.handle.worktreeSheet(up: up) }
                app.hub.askWorktree(branch: branch, in: folder, command: command)
            case .gauge(let terminal, let agent, let model, let context):
                // In the window that holds the terminal.
                app.session(holding: Int(terminal))?.hub.gauges.tell(
                    terminal: Int(terminal), agent: agent, gauge: GaugeFact(model: model, context: context))
            case .newWindow(let window, _, _, let activate): app.openPlanned(window: Int(window), activate: activate)
            case .probeNotice(let probe, let subtitle, let title, let body):
                app.notices?.post(probe: probe, subtitle: subtitle, title: title, body: body)
            case .probeOffer(let terminal, let words, let text):
                app.session(holding: Int(terminal))?.hub.offerFile(terminal: Int(terminal), text: text, words: words)
            case .probePaused(let words): app.hub.probePaused(words: words)
            case .driven(let terminal, let typing, let agent):
                app.session(holding: Int(terminal))?.hub.driven(terminal: Int(terminal), typing: typing, agent: agent)
            case .probesChanged: probesChanged()
            case .welcomeDemo(let terminal, let show): welcomeDemo(terminal: Int(terminal), show: show)
            case .welcomeEnded(let terminal, let note): welcomeEnded(terminal: Int(terminal), note: note)
            case .welcomeAskPermission: askNotificationPermission()
            case .welcomePanel(let terminal, let open): welcomePanel(terminal: Int(terminal), open: open)
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
                FileHandle.standardError.write(Data("another Wormhole serves this data folder: it was asked to come forward\n".utf8))  // text: a log line
                terminateUnattended()
            } catch {
                FileHandle.standardError.write(Data("hooks: the socket is not served (\(error)); agents' hooks will not arrive\n".utf8))  // text: a log line
            }
        }
    }
#else
    /// Without the core there is no socket to serve.
    @MainActor
    final class HookHost {
        func settingsChanged(profile: String) {}
        func revokeApproval(_ number: UInt64, stored: Bool, profile: String) {}
        func stop() {}
        @discardableResult func openFromSystem(_ urls: [URL], in window: Int) -> Int { 0 }
    }

    extension AppController {
        func startHooks() {}
    }
#endif
