import Testing
import WorkspaceModel

import PanesModel

@Suite struct TabBarModelTests {
    private func item(_ id: Int) -> TabItem { TabItem(id: id, name: "t\(id)", activity: id == 3 ? .needsInput : .idle) }

    @Test func theBarShowsOnceThereIsSomewhereToSwitchOrMoreThanOneTab() {
        let one = [TabSet(groupID: nil, label: TabSets.ungrouped, members: [1])]
        #expect(TabBarModel.make(sets: one, current: 0, selected: 1, terminal: item).isShown == false)
        let several = [TabSet(groupID: nil, label: TabSets.ungrouped, members: [1, 2])]
        #expect(TabBarModel.make(sets: several, current: 0, selected: 1, terminal: item).isShown)
        let two = [TabSet(groupID: "a", label: "A", members: [1]), TabSet(groupID: nil, label: TabSets.ungrouped, members: [2])]
        let bar = TabBarModel.make(sets: two, current: 0, selected: 1, terminal: item)
        #expect(bar.isShown && bar.showsSwitcher && bar.tabs.map(\.id) == [1] && bar.setLabel == "A")
    }

    @Test func anotherSetsTerminalThatNeedsYouLightsTheSwitcher() {
        let sets = [TabSet(groupID: "a", label: "A", members: [1, 2]), TabSet(groupID: "b", label: "B", members: [3])]
        #expect(TabBarModel.make(sets: sets, current: 0, selected: 1, terminal: item).alert)
        #expect(TabBarModel.make(sets: sets, current: 1, selected: 3, terminal: item).alert == false, "its own set is the bar's tabs")
        let cards = TabBarModel.make(sets: sets, current: 0, selected: 1, terminal: item).cards
        #expect(cards.map(\.current) == [true, false] && cards[1].alert && cards[0].count == 2)
    }
}
