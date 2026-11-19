// The tab bar's sets asked of the core (`tabSets*` functions, `view::sidebar::tabs`): the window hands over the sets it lists and
// draws the answer.

import PanesModel
import WormholeBindings

public struct CoreTabSetRules: TabSetRules {
    public init() {}

    private func records(_ sets: [TabSet]) -> [TabSetRecord] {
        sets.map { TabSetRecord(label: $0.label, members: $0.members.map(Int64.init)) }
    }

    public func current(_ sets: [TabSet], containing terminal: Int) -> Int? {
        tabSetsCurrent(setsNow: records(sets), terminal: Int64(terminal)).map(Int.init)
    }

    public func step(_ sets: [TabSet], from current: Int, forward: Bool) -> Int? {
        guard current >= 0 else { return nil }
        return tabSetsStep(setsNow: records(sets), current: UInt32(current), forward: forward).map(Int.init)
    }

    public func numbered(_ sets: [TabSet], _ number: Int) -> Int? {
        guard number >= 0 else { return nil }
        return tabSetsNumbered(setsNow: records(sets), number: UInt32(number)).map(Int.init)
    }

    public func search(_ sets: [TabSet], _ query: String, name: (Int) -> String) -> [TabSets.Hit] {
        let names = sets.flatMap(\.members).map { TabNameRecord(id: Int64($0), name: name($0)) }
        return tabSetsSearch(setsNow: records(sets), query: query, names: names).map { hit in
            hit.terminal.map { TabSets.Hit.terminal(Int($0), set: Int(hit.set)) } ?? .set(Int(hit.set))
        }
    }
}
