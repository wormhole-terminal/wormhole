// What the Notifications page asks of the notifier: the permission as macOS holds it and a test that says what came of it
// (`notifications::send_test`): it asks macOS first when that has not been asked, waits for the answer, and says when it
// was no, or never came. A test instance's notifier is the log: nothing real is shown or asked there.

#if WITH_UNIFFI
    import PlatformModel
    import SettingsModel

    extension NoticeFeed {
        /// The page's view of the permission and the test, now; asks again shortly while macOS is still looking.
        func publishStatus() {
            var status = NotifyStatus()
            status.permission =
                switch notifier.permission {
                case .checking: .checking
                case .notAsked: .notAsked
                case .allowed(let timeSensitive): .allowed(timeSensitive: timeSensitive)
                case .denied: .denied
                case .unavailable: .outsideBundle
                }
            status.test = test
            status.timeSensitive = notifier.deliversTimeSensitive
            app.settingsModel.notifyStatus = status
            if case .checking = notifier.permission {
                // one-shot: the system answers its question about the permission a moment after launch
                Task { @MainActor [weak self] in
                    try? await Task.sleep(for: .milliseconds(500))
                    self?.publishStatus()
                }
            }
        }

        /// "Send test": the banner goes once macOS allows it; the row says waiting, sent, denied or unanswered.
        func runTest(name: String, settings: NoticeSettings) {
            testTask?.cancel()
            test = .waiting
            publishStatus()
            testTask = Task { @MainActor [weak self] in
                guard let self else { return }
                if case .notAsked = notifier.permission { notifier.askPermission() }
                // macOS answers its question when the person does: look twice a second for half a minute.
                for _ in 0..<60 {
                    switch notifier.permission {
                    case .checking, .notAsked: break
                    default: return finishTest(name: name, settings: settings)
                    }
                    try? await Task.sleep(for: .milliseconds(500))
                    if Task.isCancelled { return }
                }
                test = .unanswered
                publishStatus()
            }
        }

        private func finishTest(name: String, settings: NoticeSettings) {
            if case .denied = notifier.permission {
                test = .denied
            } else {
                sendTest(name: name, settings: settings)
                test = .sent
            }
            publishStatus()
        }
    }
#endif
