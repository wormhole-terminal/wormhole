// The centre's one delegate. Responses (a click, a button, Reply's text) arrive on a private queue and go to the
// main actor over the stream. Without `willPresent` macOS keeps banners away while Wormhole is in front; whether
// one is wanted then is the app's decision (`notify_in_front`), made before it is posted.

import PlatformModel
import UserNotifications

final class NoticeDelegate: NSObject, UNUserNotificationCenterDelegate, @unchecked Sendable {
    private let continuation: AsyncStream<NoticeResponse>.Continuation

    init(_ continuation: AsyncStream<NoticeResponse>.Continuation) { self.continuation = continuation }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse
    ) async {
        let action = response.actionIdentifier == UNNotificationDefaultActionIdentifier ? nil : response.actionIdentifier
        let reply = (response as? UNTextInputNotificationResponse)?.userText
        continuation.yield(NoticeResponse(id: response.notification.request.identifier, action: action, reply: reply))
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter, willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .list, .sound]
    }
}
