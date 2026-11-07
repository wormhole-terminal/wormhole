// The words of the tab bar, the group switcher and the profile chip (`wormhole-text` sidebar catalogs).

import Foundation
import WordsModel

extension PanesText {
    static var closeTerminal: String { Words.text("tab-close", fallback: "Close terminal") }
    static func tabLabel(_ name: String, selected: Bool) -> String {
        selected
            ? Words.text("tab-spoken-selected", fallback: "\(name), tab, selected")
            : Words.text("tab-spoken", fallback: "\(name), tab")
    }
    static func switcherLabel(_ group: String, alert: Bool) -> String {
        alert
            ? Words.text("tab-switcher-spoken-alert", fallback: "Group \(group), another group needs you")
            : Words.text("tab-switcher-spoken", fallback: "Group \(group), switch group")
    }
    static var switcherSearch: String { Words.text("tab-switcher-search", fallback: "Go to group or terminal…") }
    static var noMatches: String { Words.text("tab-switcher-no-matches", fallback: "No matches") }
    static var noTerminals: String { Words.text("tab-switcher-no-terminals", fallback: "No terminals") }
    static var newGroup: String { Words.text("tab-switcher-new-group", fallback: "New group") }
    static var showGroupsInSidebar: String { Words.text("tab-switcher-show-in-sidebar", fallback: "Show all groups in the sidebar") }
    static func groupNote(count: Int) -> String {
        count == 1 ? Words.text("tab-switcher-group-one", fallback: "group · 1 terminal") : Words.text("tab-switcher-group-many", fallback: "group · \(count) terminals")
    }
    static func cardLabel(_ group: String, count: Int, current: Bool, alert: Bool) -> String {
        [group, "\(count) \(count == 1 ? "terminal" : "terminals")", current ? "current" : nil, alert ? "needs you" : nil].compactMap { $0 }.joined(separator: ", ")
    }
    static func profileChip(_ name: String) -> String { Words.text("profile-chip-spoken", fallback: "Profile \(name), switch profile") }
    static var switchTo: String { Words.text("profile-menu-switch-to", fallback: "Switch Wormhole to") }
    static var openWindowIn: String { Words.text("profile-menu-open-in", fallback: "Open a window in…") }
    static var moveWindowTo: String { Words.text("profile-menu-move-to", fallback: "Move this window to…") }
    static var newProfile: String { Words.text("profile-menu-new", fallback: "New profile…") }
    static var manageProfiles: String { Words.text("profile-menu-manage", fallback: "Manage profiles…") }
    static var settings: String { Words.text("profile-menu-settings", fallback: "Settings…") }
}
