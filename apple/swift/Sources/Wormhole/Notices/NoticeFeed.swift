// Banners, the Dock's bounce and VoiceOver's words for what the terminals do. The rules are core's (`NoticeCoordinator`
// over `PlatformBridge.makeNoticeRules()`); this feeds them a look at the terminals when one changed (the workspace's
// state is observed, never polled), carries the answers to the platform and takes a click on a banner back to the window.
// In a test instance (`--background`) the banners go to the log and nothing bounces or speaks.

#if WITH_UNIFFI
    import AppKit
    import DesignSystem
    import Observation
    import PlatformBridge
    import PlatformModel
    import PlatformUI

    @MainActor
    final class NoticeFeed {
        private let coordinator: NoticeCoordinator
        let notifier: any Notifying
        private unowned let app: AppController

        init(app: AppController) {
            self.app = app
            notifier = SystemNotifier.make(policy: app.policy)
            coordinator = NoticeCoordinator(rules: PlatformBridge.makeNoticeRules(), notifier: notifier)
            coordinator.onOutcome = { [weak app] in app?.carryOut($0) }
            if app.policy.touchesSystem { coordinator.onBounce = { NSApp.requestUserAttention(.informationalRequest) } }
            // A test instance keeps the words for `a11y said` (`Announcements.speaks`).
            coordinator.onSay = { text, urgent in Announcements.post(text, urgent: urgent) }
            coordinator.listen()
            follow()
        }

        func sendTest(name: String, settings: NoticeSettings) { coordinator.sendTest(name: name, settings: settings) }

        /// `wh notify`: a notification in the caller's words, about a terminal.
        func post(terminal: Int64, title: String, body: String) {
            notifier.post(Notice(id: "notify-\(terminal)", thread: "\(terminal)", title: title, body: body))
        }

        /// A Probe's hook asks for a notification: one thread per Probe, its name (with "extension") as the subtitle.
        func post(probe: String, subtitle: String, title: String, body: String) {
            let id = "wormhole/probe/\(probe)"
            notifier.post(Notice(id: id, thread: id, title: title, subtitle: subtitle, body: body))
        }

        /// What a click on a banner, or one of its buttons, would hand over (a test instance's banners are in the log, so this is
        /// how a driver presses one: the same path as the centre's delegate, the response going to the core's rules).
        func respond(_ response: NoticeResponse) { (notifier as? LogNotifier)?.respond(response) }

        /// The banners a test instance logged instead of posting, for the control folder's `notices`.
        var logged: [Notice] { (notifier as? LogNotifier)?.log ?? [] }

        /// A look at the terminals now, and again whenever the workspace's state changes.
        private func follow() {
            let tabs = withObservationTracking { app.noticeTabs } onChange: { [weak self] in
                Task { @MainActor in self?.follow() }
            }
            let selected = app.hub.state.selected.map(Int64.init)
            coordinator.look(tabs, settings: app.settingsNotice, appActive: NSApp.isActive)
            if let selected { coordinator.dismiss(tab: selected) }
        }
    }

    extension AppController {
        /// Every terminal as the rules read it.
        var noticeTabs: [NoticeTab] {
            let monotonic = UInt64(ProcessInfo.processInfo.systemUptime * 1000)
            return ambientTerminals.map { terminal in
                let age = UInt64(max(0, Date().timeIntervalSince(terminal.since)) * 1000)
                var tab = NoticeTab(
                    session: terminal.session, tab: terminal.session, name: terminal.name, state: terminal.state.notice,
                    sinceMs: monotonic > age ? monotonic - age : 0)
                (tab.who, tab.message, tab.folder, tab.branch) = (terminal.who, terminal.message, terminal.folder, terminal.branch)
                tab.source = terminal.source == .agent ? .agent : .plain
                let holder = session(holding: Int(terminal.session))
                tab.selected = holder?.hub.state.selected.map(Int64.init) == terminal.session
                tab.key = holder?.window?.isKeyWindow ?? false
                return tab
            }
        }

        /// What a click on a banner asks.
        func carryOut(_ outcome: NoticeOutcome) {
            switch outcome {
            case .nothing: break
            case .activate: NSApp.activate()
            case .open(let tab): show(terminal: Int(tab)); NSApp.activate()
            case .showChanges(let tab):
                show(terminal: Int(tab))
                session(holding: Int(tab))?.hub.send(.showPanelTab(.changes))
                NSApp.activate()
            case .reply(let tab, let text):
                session(holding: Int(tab))?.hub.terminals.existing(Int(tab))?.source.write(Data((text + "\r").utf8))
            }
        }
    }

    private extension AmbientState {
        var notice: NoticeState {
            switch self {
            case .idle: .idle
            case .working: .working
            case .needsYou: .needsYou
            case .done: .done
            case .failed: .failed
            case .exited: .exited
            }
        }
    }
#else
    import PlatformModel

    /// Without the core there are no rules to decide who is told what: nothing is posted.
    @MainActor
    final class NoticeFeed {
        init(app: AppController) {}
        func sendTest(name: String, settings: NoticeSettings) {}
        func post(terminal: Int64, title: String, body: String) {}
        func respond(_ response: NoticeResponse) {}
        var logged: [Notice] { [] }
    }
#endif
