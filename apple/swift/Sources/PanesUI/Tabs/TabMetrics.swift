// What the tab bar measures, each GPUI's (`style/tabs.rs`, `style/profile.rs`, `sidebar/tab_bar.rs`).

import CoreGraphics

enum TabMetrics {
    /// A tab: as wide for every one so its name's room is known (`TAB_W`), as tall as a one-line row (`TAB_H`).
    static let width: CGFloat = 172
    static let height: CGFloat = 28
    /// The open tab's fill is the text colour at this strength, a closed one's at the other (`OPEN_FILL`, `CLOSED_FILL`).
    static let openFill = 0.14
    static let closedFill = 0.05
    /// The tab's tile (`TILE_SMALL`) and what the ⌘N keycap takes from the end of a name while ⌘ is held (`HINT_W`).
    static let tile: CGFloat = 16
    static let hintWidth: CGFloat = 28
    /// The gap between the bar's parts and the hairlines that part them (`PART_RULE_H`).
    static let ruleHeight: CGFloat = 16
    /// The group switcher: its height, its longest name, the amber dot's ring (`SWITCHER_H`, `SWITCHER_NAME_MAX`, `ALERT_RING`).
    static let switcherHeight: CGFloat = 26
    static let switcherName: CGFloat = 160
    static let alertRing: CGFloat = 2
    /// The switcher's popover: its width and the gap between cards (`POPOVER_W`, `CARD_GAP`).
    static let popoverWidth: CGFloat = 340
    static let cardGap: CGFloat = 6
    /// The profile chip: its height, its avatar, the longest name it shows whole, the menu's width.
    static let chipHeight: CGFloat = 24
    static let avatar: CGFloat = 16
    static let chipName: CGFloat = 120
    static let menuWidth: CGFloat = 280
    /// The tab bar sits in the title row's height and a gap above the terminal's surface (`TAB_GAP`).
    static let gap: CGFloat = 8
}
