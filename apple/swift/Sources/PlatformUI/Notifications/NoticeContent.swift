// A `Notice` as the notification centre wants it: content with its thread, level and sound, and the categories
// whose buttons (Open, Reply…, Show Changes) the centre registers when it opens.

import PlatformModel
import UserNotifications

enum NoticeContent {
    static func content(_ notice: Notice) -> UNMutableNotificationContent {
        let content = UNMutableNotificationContent()
        content.title = notice.title
        if !notice.subtitle.isEmpty { content.subtitle = notice.subtitle }
        content.body = notice.body
        content.threadIdentifier = notice.thread
        content.interruptionLevel =
            switch notice.level {
            case .passive: .passive
            case .active: .active
            case .timeSensitive: .timeSensitive
            }
        if notice.sound { content.sound = .default }
        if notice.category != .plain { content.categoryIdentifier = notice.category.rawValue }
        return content
    }

    static func categories() -> Set<UNNotificationCategory> {
        Set([NoticeCategory.needsYou, .finished].map { category in
            let actions: [UNNotificationAction] = NoticeButton.buttons(for: category).map { button in
                switch button.kind {
                // Typing a reply needs no window: the text goes to the terminal as it is.
                case .reply:
                    UNTextInputNotificationAction(
                        identifier: button.id, title: button.label, options: [], textInputButtonTitle: "Send",
                        textInputPlaceholder: "Reply")
                case .foreground:
                    UNNotificationAction(identifier: button.id, title: button.label, options: [.foreground])
                }
            }
            return UNNotificationCategory(identifier: category.rawValue, actions: actions, intentIdentifiers: [], options: [])
        })
    }
}
