// The control of a row panel (`ControlSpec.isRowPanel`): the profile's name, start folder and git identity as
// fields, a status line for the rest. Edits of the profile go through the host when it exists (`newProfile`
// opens the form there); here the fields show the record.

import DesignSystem
import SettingsModel
import SwiftUI

struct PanelControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let name: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let profile = model.host.panelRecords().profile
        switch name {
        case "profile-name": field(profile.name)
        case "profile-start": field(profile.startFolder)
        case "profile-git": field(profile.gitName.isEmpty ? SettingsText.notSet : "\(profile.gitName) <\(profile.gitEmail)>")
        case "profile-env", "profile-logins": field(SettingsText.noneSet)
        case "profile-delete": button(String(localized: "settings-profile-delete-button", defaultValue: "Delete profile…"), "trash", .deleteProfile, danger: true)
        case "notify-permission": button(String(localized: "settings-notifications-open-system", defaultValue: "Open System Settings"), "gearshape", .openNotificationSettings)
        case "notify-test": button(String(localized: "settings-notifications-send-test", defaultValue: "Send test"), "bell", .sendTestNotification)
        case "update-status":
            HStack(spacing: Space._3) {
                Text(String(localized: "settings-updates-up-to-date", defaultValue: "Wormhole is up to date")).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textMuted)
                button(String(localized: "settings-updates-check-now", defaultValue: "Check now"), "arrow.clockwise", .checkForUpdates)
            }
        default: Text(SettingsText.notBuilt).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textFaint)
        }
    }

    private func button(_ title: String, _ symbol: String, _ action: SettingsAction, danger: Bool = false) -> some View {
        ActionButton(title: title, symbol: symbol, isDestructive: danger) { model.host.perform(action) }
    }

    private func field(_ text: String) -> some View {
        Text(text).font(.system(size: TextRole.label.size)).lineLimit(1).foregroundStyle(tokens.textPrimary)
            .padding(.horizontal, Space._3).frame(width: SettingsMetric.controlWidth, height: Metric.control, alignment: .leading)
            .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth))
    }
}
