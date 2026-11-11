// What the welcome program asks of the app besides the settings the core changes itself: the example band in the sidebar of the
// window its terminal is in, the end of the welcome, and macOS' permission for banners (asked only when a press turned one on).

#if WITH_UNIFFI
    import DesignSystem
    import Foundation
    import PlatformModel
    import SidebarUI
    import WorkspaceModel
    import WormholeBindings

    extension HookHost {
        /// The program's sidebar step is up (`show`): the core's three rows in the sidebar of the window that holds `terminal`; or
        /// it was left. Only a terminal of that window can put the band up.
        func welcomeDemo(terminal: Int, show: Bool) {
            guard let hub = app.session(holding: terminal)?.hub else { return }
            guard show else {
                if hub.exampleBand?.owner == terminal { hub.exampleBand = nil }
                return
            }
            guard hub.exampleBand?.owner != terminal else { return }
            hub.exampleBand = ExampleBand(
                owner: terminal, title: welcomeDemoTitle(), label: welcomeDemoLabel(),
                rows: { seconds, still in
                    welcomeDemoRows(elapsedMs: UInt64(seconds * 1000), still: still).map {
                        ExampleBand.Row(name: $0.name, state: $0.state.activity, detail: $0.detail, spoken: $0.spoken)
                    }
                },
                next: { seconds, still in welcomeDemoNextMs(elapsedMs: UInt64(seconds * 1000), still: still).map { Double($0) / 1000 } })
        }

        /// "How much to show" opens or closes the right panel of the window the welcome's terminal is in, as its toggle does.
        func welcomePanel(terminal: Int, open: Bool) {
            guard let hub = app.session(holding: terminal)?.hub, hub.state.panelVisible != open else { return }
            hub.send(.togglePanel)
        }

        /// The welcome is over: the band goes and a start animation still playing is cut short.
        func welcomeEnded(terminal: Int) {
            app.sessions.forEach { if $0.hub.exampleBand?.owner == terminal { $0.hub.exampleBand = nil } }
            app.aux.introPlayer.skip()
        }

        /// The terminal is closed: its band goes, and its welcome's token with it.
        func terminalClosed(_ terminal: Int) {
            welcomeEnded(terminal: terminal)
            handle.welcomeGone(tab: Int64(terminal))
        }

        /// A press turned a banner on, or said Allow: macOS is asked (it prompts only while undecided), and the page is told the
        /// answer when it comes.
        func askNotificationPermission() {
            app.notices?.notifier.askPermission()
            tellPermission()
        }

        /// The welcome's pages show what macOS says of banners: told now and each time it changes in the next while (reading it
        /// is asynchronous, and the prompt is the person's).
        func tellPermission() {
            permissionWatch?.cancel()
            permissionWatch = Task { [weak self] in
                var told = ""
                for _ in 0..<480 where !Task.isCancelled {
                    if let word = self?.app.notices?.permissionWord, word != told {
                        told = word
                        self?.handle.setNotificationPermission(word)
                    }
                    try? await Task.sleep(for: .milliseconds(250))
                }
            }
        }
    }

    extension WelcomeDemoState {
        fileprivate var activity: Activity {
            switch self {
            case .working: .working
            case .needsYou: .needsInput
            case .finished: .finished
            }
        }
    }

    extension NoticeFeed {
        /// What macOS says of banners, as the core's word for it.
        var permissionWord: String {
            switch notifier.permission {
            case .checking: "checking"
            case .notAsked: "not_asked"
            case .allowed: "allowed"
            case .denied: "denied"
            case .unavailable: "outside"
            }
        }
    }
#endif
