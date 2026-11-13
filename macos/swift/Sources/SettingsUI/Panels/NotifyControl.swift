// The Notifications page's two rows about macOS (`notifications.rs` `draw_permission`, `draw_test`): the permission with what
// to do about it, and the test with what came of it. The words are GPUI's; the state is the app's (`SettingsModel.notifyStatus`).

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

enum NotifyText {
    /// The permission row's label and its hint under it.
    static func permission(_ state: NotifyStatus.Permission) -> (title: String, hint: String) {
        switch state {
        case .checking: (Words.text("settings-notifications-asking-macos", fallback: "Asking macOS…"), "")
        case .notAsked:
            (
                Words.text("settings-notifications-not-asked-yet", fallback: "Not asked yet"),
                Words.text("settings-notifications-macos-asks-once-nothing-is-sent", fallback: "macOS asks once; nothing is sent before")
            )
        case .allowed(let timeSensitive):
            (
                Words.text("settings-notifications-allowed-in-system-settings", fallback: "Allowed in System Settings"),
                timeSensitive
                    ? Words.text("settings-notifications-banners-sounds-and-time-sensitive", fallback: "Banners, sounds and Time Sensitive")
                    : Words.text("settings-notifications-banners-and-sounds", fallback: "Banners and sounds")
            )
        case .denied:
            (
                Words.text("settings-notifications-off-in-system-settings", fallback: "Off in System Settings"),
                Words.text("settings-notifications-off-hint", fallback: "Nothing is delivered until turned on there")
            )
        case .outsideBundle:
            (
                Words.text("settings-notifications-outside-the-app-bundle", fallback: "Outside the app bundle"),
                Words.text("settings-notifications-outside-bundle-hint", fallback: "cargo run: osascript, no actions or grouping")
            )
        }
    }

    static func test(_ state: NotifyStatus.Test) -> String {
        switch state {
        case .idle: Words.text("settings-notifications-test-idle", fallback: "Shows a banner the way a terminal that needs input does")
        case .waiting: Words.text("settings-notifications-test-waiting", fallback: "Waiting for macOS. Answer its question and the test is sent")
        case .sent: Words.text("settings-notifications-test-sent", fallback: "Sent. No banner? Check Focus, and the alert style in System Settings")
        case .denied: Words.text("settings-notifications-test-denied", fallback: "Off in System Settings. Turn Wormhole on there, then try again")
        case .unanswered:
            Words.text("settings-notifications-test-unanswered", fallback: "macOS did not turn notifications on. Turn Wormhole on in System Settings, then try again")
        case .refused(let error):
            Words.text("settings-notifications-test-refused", ["error": .text(error)], fallback: "macOS refused the notification: \(error)")
        }
    }

    /// What a "needs input" row says: whether this build can deliver it as Time Sensitive.
    static func needsYou(timeSensitive: Bool) -> String {
        timeSensitive
            ? Words.text("settings-notifications-needs-you-hint", fallback: "An agent asks a question or for permission")
            : Words.text("settings-notifications-needs-you-hint-plain", fallback: "An agent asks a question or for permission. Delivered as a normal notification in this build")
    }

    static var openSystemSettings: String { Words.text("settings-notifications-open-system-settings", fallback: "Open System Settings") }
}

struct NotifyControl: View {
    let model: SettingsModel
    let name: String

    var body: some View {
        HStack(spacing: Space._2) {
            switch name {
            case "notify-permission": permission
            default: test
            }
        }
        .task { model.refreshNotify?() }
    }

    @ViewBuilder private var permission: some View {
        switch model.notifyStatus.permission {
        case .allowed, .denied: openSettings
        case .notAsked:
            ActionButton(title: Words.text("settings-notifications-allow-notifications", fallback: "Allow notifications"), symbol: Icon.notifications.rawValue) {
                model.host.perform(.askNotifications)
            }
        case .checking, .outsideBundle: EmptyView()
        }
    }

    @ViewBuilder private var test: some View {
        ActionButton(title: Words.text("settings-notifications-send-test", fallback: "Send test"), symbol: Icon.notifications.rawValue) {
            model.host.perform(.sendTestNotification)
        }
        switch model.notifyStatus.test {
        case .denied, .unanswered: openSettings
        default: EmptyView()
        }
    }

    private var openSettings: some View {
        ActionButton(title: NotifyText.openSystemSettings, symbol: Icon.settings.rawValue) { model.host.perform(.openNotificationSettings) }
    }
}
