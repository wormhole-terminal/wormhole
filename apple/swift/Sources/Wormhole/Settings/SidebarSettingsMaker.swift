// The sidebar's list from Settings → Terminals & sidebar and Motion → Status.

import SettingsModel
import SidebarUI
import DesignSystem

enum SidebarSettingsMaker {
    static func appearance(_ values: [String: JSONValue], current: SidebarAppearance) -> SidebarAppearance {
        var next = current
        next.density = values["density"]?.string.flatMap(SidebarDensity.init(rawValue:)) ?? current.density
        next.cards = values["show_groups_as_cards"]?.bool ?? current.cards
        next.fadesGroupColours = values["fade_group_colors"]?.bool ?? current.fadesGroupColours
        next.showsGroupFolders = values["show_group_folders"]?.bool ?? current.showsGroupFolders
        next.showsNotes = values["sidebar_notes"]?.bool ?? current.showsNotes
        next.showsChanges = values["sidebar_changes"]?.bool ?? current.showsChanges
        next.statusMotion = values["motion.status"]?.string.flatMap(statusMotion) ?? current.statusMotion
        return next
    }

    /// The settings' spelling (`badge-pop`, `line-roll`) of `StatusMotion`.
    static func statusMotion(_ key: String) -> StatusMotion? {
        switch key {
        case "classic": .classic
        case "crossfade": .crossfade
        case "badge-pop": .badgePop
        case "wash": .wash
        case "line-roll": .lineRoll
        default: nil
        }
    }
}
