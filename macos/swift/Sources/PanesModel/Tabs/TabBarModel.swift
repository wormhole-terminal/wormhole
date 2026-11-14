// What the tab bar shows (`sidebar/tab_bar.rs`): the selected terminal's set as tabs, the group switcher, and
// whether the bar shows at all. Pure values from the sets and the terminals.

import Foundation
import WorkspaceModel

/// What a terminal calls for, by its tile's urgency (`switcher.rs`): the amber dot for one that needs you, the red one for a
/// failure; nothing for the rest.
public enum TabAlert: Sendable, Equatable {
    case none, attention, failed

    /// What the core's urgency is on the bar: needs you is the amber dot, failed the red one, the rest nothing.
    public init(_ urgency: Urgency?) {
        switch urgency {
        case .needsYou?: self = .attention
        case .failed?: self = .failed
        default: self = .none
        }
    }

    /// Louder first.
    var rank: Int {
        switch self {
        case .attention: 0
        case .failed: 1
        case .none: 2
        }
    }
}

public struct TabItem: Sendable, Equatable, Identifiable {
    public var id: TerminalID
    public var name: String
    public var activity: Activity
    /// The ⌘1–9 number while the hints show.
    public var number: Int?
    /// What VoiceOver hears, in the core's words ("build, working, selected, 2 of 4"); nil says the name only.
    public var spoken: String?
    /// What its tile's urgency says (the window sets it from the row's own tile; the sample leaves it to `activity`).
    public var alert: TabAlert?

    /// The alert: the tile's, else what `activity` says.
    var calls: TabAlert {
        alert ?? (activity == .needsInput ? .attention : activity == .failed ? .failed : .none)
    }

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
    /// A terminal of the group needs you or failed: the card's dot, amber or red.
    public var alert: Bool
    public var alertKind: TabAlert
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
    public var alertKind: TabAlert
    /// The bar shows once there is more than one set, or the current set has more than one terminal.
    public var isShown: Bool

    /// The group a new terminal from the bar's + goes into: the selected terminal's (nil: none).
    public var currentGroup: GroupID? { sets.first { $0.members.contains { $0 == selected } }?.groupID }

    /// The bar for `sets` (as the core lists them, `TabSetFacts`) with `current` the set the selected terminal is in.
    /// `alerts`: each set's alert and the closed switcher's dot as the core decides them (`others_need_you`); without the
    /// core (the sample) they come from the tabs' own.
    public static func make(
        sets: [TabSet], current: Int, selected: TerminalID?, terminal: (TerminalID) -> TabItem, pinned: (TabSet) -> Bool = { _ in false },
        alerts: (sets: [TabAlert], others: TabAlert)? = nil
    ) -> TabBarModel {
        let members = sets.indices.contains(current) ? sets[current].members : []
        let showsSwitcher = sets.count > 1
        let cards = sets.enumerated().map { at, set in
            let items = set.members.map(terminal)
            // Needs-you is the louder of the two (core `Urgency` order).
            let own: TabAlert = items.contains { $0.calls == .attention } ? .attention : items.contains { $0.calls == .failed } ? .failed : .none
            let kind = alerts.map { $0.sets.indices.contains(at) ? $0.sets[at] : .none } ?? own
            return SetCard(
                id: at, label: set.label, activities: items.map(\.activity), current: at == current,
                alert: kind != .none, alertKind: kind, pinned: pinned(set))
        }
        return TabBarModel(
            tabs: members.map(terminal), selected: selected, setLabel: sets.indices.contains(current) ? sets[current].label : "",
            cards: cards, sets: sets, names: Dictionary(sets.flatMap(\.members).map { ($0, terminal($0).name) }, uniquingKeysWith: { first, _ in first }),
            showsSwitcher: showsSwitcher, alert: alerts.map { $0.others != .none } ?? cards.contains { !$0.current && $0.alert },
            alertKind: alerts?.others ?? cards.filter { !$0.current }.map(\.alertKind).min { $0.rank < $1.rank } ?? .none,
            isShown: showsSwitcher || members.count > 1)
    }
}
