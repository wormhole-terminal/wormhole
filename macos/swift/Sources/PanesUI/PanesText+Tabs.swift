// The words of the tab bar, the group switcher and the profile chip (`wormhole-text` sidebar catalogs).

import Foundation
import WordsModel

extension PanesText {
    static var newTerminal: String { Words.text("tab-new", fallback: "New terminal") }
    static var newTerminalWith: String { Words.text("tab-new-with", fallback: "New terminal with") }
    static func newTerminalIn(_ group: String) -> String {
        Words.text("tab-switcher-new-in-group", ["group": .text(group)], fallback: "New terminal in \(group)")
    }
    static var closeTerminal: String { Words.text("tab-close", fallback: "Close terminal") }
    static func tabLabel(_ name: String, selected: Bool) -> String {
        selected
            ? Words.text("tab-spoken-selected", ["name": .text(name)], fallback: "\(name), tab, selected")
            : Words.text("tab-spoken", ["name": .text(name)], fallback: "\(name), tab")
    }
    static func switcherLabel(_ group: String, alert: Bool) -> String {
        alert
            ? Words.text("tab-switcher-spoken-alert", ["group": .text(group)], fallback: "Group \(group), another group needs you")
            : Words.text("tab-switcher-spoken", ["group": .text(group)], fallback: "Group \(group), switch group")
    }
    static var switcherSearch: String { Words.text("tab-switcher-search", fallback: "Go to group or terminal…") }
    static var noMatches: String { Words.text("tab-switcher-no-matches", fallback: "No matches") }
    static var noTerminals: String { Words.text("tab-switcher-no-terminals", fallback: "No terminals") }
    static var newGroup: String { Words.text("tab-switcher-new-group", fallback: "New group") }
    static var showGroupsInSidebar: String { Words.text("tab-switcher-show-in-sidebar", fallback: "Show all groups in the sidebar") }
    static func groupNote(count: Int) -> String {
        Words.text("tab-switcher-group", ["count": .whole(count)], fallback: count == 1 ? "group · 1 terminal" : "group · \(count) terminals")
    }
    static func cardLabel(_ group: String, count: Int, current: Bool, alert: Bool) -> String {
        let terminals = Words.text(
            "tab-switcher-card-terminals", ["count": .whole(count)], fallback: "\(count) \(count == 1 ? "terminal" : "terminals")")
        return [
            group, terminals, current ? Words.text("tab-switcher-card-current", fallback: "current") : nil,
            alert ? Words.text("tab-switcher-card-alert", fallback: "needs you") : nil,
        ].compactMap { $0 }.joined(separator: ", ")
    }
    static func profileChip(_ name: String) -> String { Words.text("profile-chip-spoken", ["name": .text(name)], fallback: "Profile \(name), switch profile") }
    static var switchTo: String { Words.text("profile-menu-switch-to", fallback: "Switch Wormhole to") }
    static var openWindowIn: String { Words.text("profile-menu-open-in", fallback: "Open a window in…") }
    static var moveWindowTo: String { Words.text("profile-menu-move-to", fallback: "Move this window to…") }
    static var newProfile: String { Words.text("profile-menu-new", fallback: "New profile…") }
    static var manageProfiles: String { Words.text("profile-menu-manage", fallback: "Manage profiles…") }
    static var settings: String { Words.text("profile-menu-settings", fallback: "Settings…") }
}
