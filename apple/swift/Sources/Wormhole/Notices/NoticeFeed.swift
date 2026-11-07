// Banners, the Dock's bounce and VoiceOver's words for what the terminals do. The rules are core's (`NoticeCoordinator`
// over `PlatformBridge.makeNoticeRules()`); this feeds them a look at the terminals when one changed (the workspace's
// state is observed, never polled), carries the answers to the platform and takes a click on a banner back to the window.
// In a test instance (`--background`) the banners go to the log and nothing bounces or speaks.

#if WITH_UNIFFI
    import AppKit
    import Observation
    import PlatformBridge
    import PlatformModel
    import PlatformUI

    @MainActor
    final class NoticeFeed {
        private let coordinator: NoticeCoordinator
        private let notifier: any Notifying
        private unowned let app: AppController

        init(app: AppController) {
            self.app = app
            notifier = SystemNotifier.make(policy: app.policy)
            coordinator = NoticeCoordinator(rules: PlatformBridge.makeNoticeRules(), notifier: notifier)
            coordinator.onOutcome = { [weak app] in app?.carryOut($0) }
            if app.policy.touchesSystem {
                coordinator.onBounce = { NSApp.requestUserAttention(.informationalRequest) }
                coordinator.onSay = { text, urgent in
                    NSAccessibility.post(
                        element: NSApp as Any, notification: .announcementRequested,
                        userInfo: [
                            .announcement: text,
                            .priority: (urgent ? NSAccessibilityPriorityLevel.high : .medium).rawValue,
                        ])
                }
            }
            coordinator.listen()
            follow()
        }

        func sendTest(name: String, settings: NoticeSettings) { coordinator.sendTest(name: name, settings: settings) }

        /// `wh notify`: a notification in the caller's words, about a terminal.
        func post(terminal: Int64, title: String, body: String) {
            notifier.post(Notice(id: "notify-\(terminal)", thread: "\(terminal)", title: title, body: body))
        }

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
        var logged: [Notice] { [] }
    }
#endif
