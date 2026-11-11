// The control of a row panel (`ControlSpec.isRowPanel`): the profile's name, start folder and git identity as
// fields, a status line for the rest. Edits of the profile go through the host when it exists (`newProfile`
// opens the form there); here the fields show the record.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct PanelControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let name: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        switch name {
        case "profile-name", "profile-start", "profile-env", "profile-git", "profile-logins": ProfileControl(draft: model.profile, name: name)
        case "cliTool": CommandLineControl(model: model)
        case "agent-mcp": McpControl(command: model.host.panelRecords().mcpCommand)
        // Who may drive Wormhole from outside: nothing beyond listing, a question the first time, or everything while it runs.
        case "agentAccess":
            OptionPicker(
                options: [("off", SettingsText.accessOff), ("ask", SettingsText.accessAsk), ("open", SettingsText.accessOpen)]
                    .map { SettingOption(value: .string($0.0), label: $0.1) },
                selected: model.value("agent_access") == .null ? .string("ask") : model.value("agent_access"), label: entry.label,
                segmented: true
            ) { model.set("agent_access", $0) }
        case "profile-delete":
            ActionButton(title: SettingsText.deleteProfile, symbol: Icon.trash.rawValue, isDestructive: true) { model.confirmsProfileDelete = true }
                .disabled(model.host.panelRecords().profiles.first(where: \.isActive)?.isDefault ?? true)
        case "notify-permission": button(Words.text("settings-notifications-open-system", fallback: "Open System Settings"), Icon.settings.rawValue, .openNotificationSettings)
        case "notify-test": button(Words.text("settings-notifications-send-test", fallback: "Send test"), Icon.notifications.rawValue, .sendTestNotification)
        default: Text(SettingsText.notBuilt).textRole(.label).foregroundStyle(tokens.textFaint)
        }
    }

    private func button(_ title: String, _ symbol: String, _ action: SettingsAction, danger: Bool = false) -> some View {
        ActionButton(title: title, symbol: symbol, isDestructive: danger) { model.host.perform(action) }
    }

}
