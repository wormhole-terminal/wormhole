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
    import WordsModel
    import WorkspaceBridge
    import WormholeBindings

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

        /// Whether "needs input" may go out Time Sensitive: the build is signed for it and the person allows it
        /// (`notifications::time_sensitive`).
        private var timeSensitive: Bool { (notifier as? SystemNotifier)?.deliversTimeSensitive ?? false }

        /// "Send test" never ends silent (`notifications/test_banner.rs`): macOS is asked first when it was not, and `report`
        /// says what happened: off in System Settings, never answered, or handed to the centre.
        func sendTest(name: String, settings: NoticeSettings, report: @escaping @MainActor (String) -> Void) {
            guard let system = notifier as? SystemNotifier else {
                coordinator.sendTest(name: name, settings: settings)
                return
            }
            Task { @MainActor in
                switch system.permission {
                case .notAsked, .checking:
                    report(Words.text("settings-notifications-test-waiting", fallback: "Waiting for macOS. Answer its question and the test is sent"))
                    await system.askPermissionAndWait()
                    // Still not allowed or refused after the question: it was dismissed.
                    if [.notAsked, .checking].contains(system.permission) {
                        return report(Words.text("settings-notifications-test-unanswered", fallback: "macOS did not turn notifications on. Turn Wormhole on in System Settings, then try again"))
                    }
                    deliver(name, settings, system, report)
                default: deliver(name, settings, system, report)
                }
            }
        }

        private func deliver(_ name: String, _ settings: NoticeSettings, _ system: SystemNotifier, _ report: @MainActor (String) -> Void) {
            if case .denied = system.permission {
                return report(Words.text("settings-notifications-test-denied", fallback: "Off in System Settings. Turn Wormhole on there, then try again"))
            }
            coordinator.sendTest(name: name, settings: settings, timeSensitive: timeSensitive)
            report(Words.text("settings-notifications-test-sent", fallback: "Sent. No banner? Check Focus, and the alert style in System Settings"))
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
            coordinator.look(tabs, settings: app.settingsNotice, appActive: NSApp.isActive, timeSensitive: timeSensitive)
            // Looked at (`Event::Seen`): selected in the key window of an active app. A banner for it posted while Wormhole is in the
            // background stays until the person comes back (the look that follows the window's focus then takes it down).
            if let selected, NSApp.isActive, app.front.window?.isKeyWindow == true { coordinator.dismiss(tab: selected) }
        }
    }

    extension AppController {
        /// Every terminal as the rules read it: its row, and what the core keeps for the notifications (who tells its
        /// status, its latest notification in its own words, the last command's failure, the program in front).
        var noticeTabs: [NoticeTab] {
            let monotonic = UInt64(ProcessInfo.processInfo.systemUptime * 1000)
            // Read so that a new notification, which may change nothing on a row, looks again.
            _ = sessions.map { $0.hub.state.all.map(\.noticeSerial) }
            let core = coreNoticeFacts(nowMs: monotonic)
            return ambientTerminals.map { terminal in
                let age = UInt64(max(0, Date().timeIntervalSince(terminal.since)) * 1000)
                var tab = NoticeTab(
                    session: terminal.session, tab: terminal.session, name: terminal.name, state: terminal.state.notice,
                    sinceMs: monotonic > age ? monotonic - age : 0)
                (tab.who, tab.message, tab.folder, tab.branch) = (terminal.who, terminal.message, terminal.folder, terminal.branch)
                tab.source = terminal.source == .agent ? .agent : .plain
                if let facts = core[terminal.session] { tab.take(facts) }
                let holder = session(holding: Int(terminal.session))
                tab.selected = holder?.hub.state.selected.map(Int64.init) == terminal.session
                tab.key = holder?.window?.isKeyWindow ?? false
                return tab
            }
        }

        /// What the core keeps of every window's terminals for the notifications, by id (times on the monotonic clock `nowMs`).
        func coreNoticeFacts(nowMs: UInt64) -> [Int64: NoticeFactsRecord] {
            let facts = sessions.flatMap { ($0.hub.store.hooks as? RustWorkspaceBackend)?.core.noticeFacts(nowMs: nowMs) ?? [] }
            return Dictionary(facts.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        }

        /// A program that reports its own status (OSC 7501) is worded under its name, as an agent is, on every ambient surface.
        func namedByReports(_ terminals: [AmbientTerminal]) -> [AmbientTerminal] {
            let told = coreNoticeFacts(nowMs: 0)
            return terminals.map { terminal in
                guard let facts = told[terminal.session], facts.source == .program else { return terminal }
                var named = terminal
                (named.source, named.who) = (.program, facts.who ?? terminal.who)
                return named
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

    private extension NoticeTab {
        /// What the core keeps of the terminal for the notifications, over what its row says.
        mutating func take(_ facts: NoticeFactsRecord) {
            source = switch facts.source {
            case .plain: .plain
            case .agent: .agent
            case .program: .program
            }
            who = facts.who ?? who
            reported = facts.reported
            notice = facts.notice.map { ProgramNotice(serial: $0.serial, title: $0.title, body: $0.body) }
            failure = facts.failure.map {
                Failure(serial: $0.serial, code: $0.code, atMs: $0.atMs, command: $0.command, tookMs: $0.tookMs, lastLine: $0.lastLine)
            }
            foreground = facts.foreground.map { Foreground(command: $0.command, startedUnixMs: $0.startedUnixMs) }
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
        func sendTest(name: String, settings: NoticeSettings, report: @escaping @MainActor (String) -> Void) {}
        func respond(_ response: NoticeResponse) {}
        var logged: [Notice] { [] }
    }
#endif
