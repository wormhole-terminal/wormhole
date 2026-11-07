// The words of the tab bar, the group switcher and the profile chip (`wormhole-text` sidebar catalogs).

import Foundation

extension PanesText {
    static var closeTerminal: String { String(localized: "tab-close", defaultValue: "Close terminal") }
    static func tabLabel(_ name: String, selected: Bool) -> String {
        selected
            ? String(localized: "tab-spoken-selected", defaultValue: "\(name), tab, selected")
            : String(localized: "tab-spoken", defaultValue: "\(name), tab")
    }
    static func switcherLabel(_ group: String, alert: Bool) -> String {
        alert
            ? String(localized: "tab-switcher-spoken-alert", defaultValue: "Group \(group), another group needs you")
            : String(localized: "tab-switcher-spoken", defaultValue: "Group \(group), switch group")
    }
    static var switcherSearch: String { String(localized: "tab-switcher-search", defaultValue: "Go to group or terminal…") }
    static var noMatches: String { String(localized: "tab-switcher-no-matches", defaultValue: "No matches") }
    static var noTerminals: String { String(localized: "tab-switcher-no-terminals", defaultValue: "No terminals") }
    static var newGroup: String { String(localized: "tab-switcher-new-group", defaultValue: "New group") }
    static var showGroupsInSidebar: String { String(localized: "tab-switcher-show-in-sidebar", defaultValue: "Show all groups in the sidebar") }
    static func groupNote(count: Int) -> String {
        count == 1 ? String(localized: "tab-switcher-group-one", defaultValue: "group · 1 terminal") : String(localized: "tab-switcher-group-many", defaultValue: "group · \(count) terminals")
    }
    static func cardLabel(_ group: String, count: Int, current: Bool, alert: Bool) -> String {
        [group, "\(count) \(count == 1 ? "terminal" : "terminals")", current ? "current" : nil, alert ? "needs you" : nil].compactMap { $0 }.joined(separator: ", ")
    }
    static func profileChip(_ name: String) -> String { String(localized: "profile-chip-spoken", defaultValue: "Profile \(name), switch profile") }
    static var switchTo: String { String(localized: "profile-menu-switch-to", defaultValue: "Switch Wormhole to") }
    static var openWindowIn: String { String(localized: "profile-menu-open-in", defaultValue: "Open a window in…") }
    static var moveWindowTo: String { String(localized: "profile-menu-move-to", defaultValue: "Move this window to…") }
    static var newProfile: String { String(localized: "profile-menu-new", defaultValue: "New profile…") }
    static var manageProfiles: String { String(localized: "profile-menu-manage", defaultValue: "Manage profiles…") }
    static var settings: String { String(localized: "profile-menu-settings", defaultValue: "Settings…") }
}
