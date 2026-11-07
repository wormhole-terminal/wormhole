// Sample tab bars and profiles for previews and snapshots.

import PanesModel
import SwiftUI
import WorkspaceModel

public enum TabSamples {
    private static let terminals: [TabItem] = [
        TabItem(id: 1, name: "zsh", number: 1), TabItem(id: 2, name: "sidebar polish", activity: .working, number: 2),
        TabItem(id: 3, name: "diff perf", activity: .needsInput, number: 3), TabItem(id: 4, name: "rate limits", activity: .finished, number: 4),
        TabItem(id: 5, name: "a long terminal name that cannot fit its tab", number: 5),
    ]

    public static let sets = [
        TabSet(groupID: "agents", label: "Agents", members: [2, 3, 4]),
        TabSet(groupID: "docs", label: "Docs", members: []),
        TabSet(groupID: nil, label: TabSets.ungrouped, members: [1, 5]),
    ]

    public static func bar(current: Int = 0, selected: TerminalID = 2) -> TabBarModel {
        TabBarModel.make(sets: sets, current: current, selected: selected, terminal: { id in terminals.first { $0.id == id } ?? TabItem(id: id, name: "?") }, pinned: { $0.label == "Docs" })
    }

    public static let profiles: [ProfileChoice] = [
        ProfileChoice(id: "work", name: "Work", letter: "W", colour: .blue, current: true, here: true),
        ProfileChoice(id: "private", name: "Private", letter: "P", colour: .green, current: false, here: false),
    ]
}
