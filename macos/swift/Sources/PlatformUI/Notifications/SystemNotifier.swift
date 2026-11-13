// The notification centre of the bundled app (`platform/macos/notifications.rs`): banners with an interruption
// level (Time Sensitive for "needs input" when the build is entitled), a thread per terminal, sound, and the
// Open / Reply… / Show Changes buttons. The centre aborts a process that isn't an app bundle, so it is never
// touched without one, nor in a test instance: `make` returns the log notifier there.

import Foundation
import PlatformModel
import UserNotifications
import WordsModel

@MainActor
public final class SystemNotifier: Notifying {
    public private(set) var permission: NoticePermission = .checking
    public let responses: AsyncStream<NoticeResponse>
    private let center: UNUserNotificationCenter
    private let delegate: NoticeDelegate
    private var asked = false

    /// The notifier a run uses: the centre in a bundled app, the log in a test instance, and a log that says
    /// `unavailable` for a bare executable (`swift run`).
    public static func make(
        policy: PlatformPolicy = .current(), bundled: Bool = Bundle.main.bundleIdentifier != nil
            && Bundle.main.bundleURL.pathExtension == "app"
    ) -> any Notifying {
        switch NoticeRoute.choose(policy: policy, bundled: bundled) {
        case .center: SystemNotifier()
        case .log, .off: LogNotifier()
        }
    }

    init() {
        let (stream, continuation) = AsyncStream<NoticeResponse>.makeStream()
        responses = stream
        delegate = NoticeDelegate(continuation)
        center = UNUserNotificationCenter.current()
        center.delegate = delegate
        center.setNotificationCategories(NoticeContent.categories())
        refreshPermission()
    }

    /// Whether Time Sensitive is both signed for and allowed by the person.
    public var deliversTimeSensitive: Bool {
        if case .allowed(let timeSensitive) = permission { timeSensitive && TimeSensitiveEntitlement.isGranted } else { false }
    }

    public func post(_ notice: Notice) {
        if !asked { askPermission() }
        // The same identifier replaces a terminal's earlier notification; a nil trigger delivers now.
        let request = UNNotificationRequest(identifier: notice.id, content: NoticeContent.content(notice), trigger: nil)
        center.add(request) { error in
            if let error { AppLog.log("error", "posting a notification failed: \(error.localizedDescription)") }
        }
    }

    public func remove(id: String) {
        center.removePendingNotificationRequests(withIdentifiers: [id])
        center.removeDeliveredNotifications(withIdentifiers: [id])
    }

    public func askPermission() {
        asked = true
        Task { [center] in
            _ = try? await center.requestAuthorization(options: [.alert, .sound])
            await self.refresh()
        }
    }

    /// Asks and waits for the person's answer (the test banner's press waits for it; the permission is read again after it).
    public func askPermissionAndWait() async {
        asked = true
        _ = try? await center.requestAuthorization(options: [.alert, .sound])
        await refresh()
    }

    func refreshPermission() { Task { await refresh() } }

    private func refresh() async {
        let settings = await center.notificationSettings()
        permission =
            switch settings.authorizationStatus {
            case .notDetermined: .notAsked
            case .denied: .denied
            default: .allowed(timeSensitive: settings.timeSensitiveSetting == .enabled)
            }
    }
}
