// What the tab bar shows (`sidebar/tab_bar.rs`): the selected terminal's set as tabs, the group switcher, and
// whether the bar shows at all. Pure values from the sets and the terminals.

import Foundation
import WorkspaceModel

public struct TabItem: Sendable, Equatable, Identifiable {
    public var id: TerminalID
    public var name: String
    public var activity: Activity
    /// The ⌘1–9 number while the hints show.
    public var number: Int?
    /// What VoiceOver hears, in the core's words ("build, working, selected, 2 of 4"); nil says the name only.
    public var spoken: String?

    public init(id: TerminalID, name: String, activity: Activity = .idle, number: Int? = nil, spoken: String? = nil) {
        (self.id, self.name, self.activity, self.number, self.spoken) = (id, name, activity, number, spoken)
    }
}

/// A group as the switcher's card shows it: its name, its terminals as tiles and count, ringed when open.
public struct SetCard: Sendable, Equatable, Identifiable {
    public var id: Int
    public var label: String
    public var activities: [Activity]
    public var current: Bool
    /// A terminal of the group needs input or failed: the card's amber dot.
    public var alert: Bool
    public var pinned: Bool

    public var count: Int { activities.count }
}

public struct TabBarModel: Sendable, Equatable {
    public var tabs: [TabItem]
    public var selected: TerminalID?
    public var setLabel: String
    public var cards: [SetCard]
    /// Every set and every terminal's name, for the switcher's search.
    public var sets: [TabSet]
    public var names: [TerminalID: String]
    /// The set the group switcher is drawn for: more than one set to switch between.
    public var showsSwitcher: Bool
    /// A set other than the current one has a terminal that needs you: the amber dot on the closed switcher.
    public var alert: Bool
    /// The bar shows once there is more than one set, or the current set has more than one terminal.
    public var isShown: Bool

    /// The bar for `sets` (as `TabSets.of` lists them) with `current` the set the selected terminal is in.
    public static func make(
        sets: [TabSet], current: Int, selected: TerminalID?, terminal: (TerminalID) -> TabItem, pinned: (TabSet) -> Bool = { _ in false }
    ) -> TabBarModel {
        let members = sets.indices.contains(current) ? sets[current].members : []
        let showsSwitcher = sets.count > 1
        let cards = sets.enumerated().map { at, set in
            let items = set.members.map(terminal)
            return SetCard(
                id: at, label: set.label, activities: items.map(\.activity), current: at == current,
                alert: items.contains { $0.activity == .needsInput || $0.activity == .failed }, pinned: pinned(set))
        }
        return TabBarModel(
            tabs: members.map(terminal), selected: selected, setLabel: sets.indices.contains(current) ? sets[current].label : "",
            cards: cards, sets: sets, names: Dictionary(sets.flatMap(\.members).map { ($0, terminal($0).name) }, uniquingKeysWith: { first, _ in first }),
            showsSwitcher: showsSwitcher, alert: cards.contains { !$0.current && $0.alert },
            isShown: showsSwitcher || members.count > 1)
    }
}
