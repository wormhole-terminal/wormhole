// What the notification rules read of Settings → Notifications and the Dock (`notify_*`, `dock_bounce`).

import PlatformModel
import SettingsModel

enum NoticeSettingsMaker {
    static func settings(_ values: [String: JSONValue]) -> NoticeSettings {
        var settings = NoticeSettings()
        settings.needsYou = values["notify_needs_you"]?.bool ?? settings.needsYou
        settings.agentDone = values["notify_agent_done"]?.bool ?? settings.agentDone
        settings.commandDone = values["notify_command_done"]?.bool ?? settings.commandDone
        settings.commandAfterSecs = values["notify_command_after"]?.number.map { UInt32(max(0, $0)) } ?? settings.commandAfterSecs
        settings.commandFailed = values["notify_command_failed"]?.bool ?? settings.commandFailed
        settings.bell = values["notify_bell"]?.bool ?? settings.bell
        settings.onlyUnseen = values["notify_only_unseen"]?.bool ?? settings.onlyUnseen
        settings.bannerInFront = (values["notify_in_front"]?.string ?? "none") != "none"
        settings.sound =
            switch values["notify_sound"]?.string {
            case "all": .all
            case "none": .none
            default: .needsYou
            }
        settings.groupBursts = values["notify_group_bursts"]?.bool ?? settings.groupBursts
        settings.dockBounce = values["dock_bounce"]?.bool ?? settings.dockBounce
        return settings
    }
}
