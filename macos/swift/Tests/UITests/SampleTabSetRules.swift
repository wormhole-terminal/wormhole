import PanesModel

/// Without the FFI the window tests reach a tab set through a stand-in for the core's rules (`CoreTabSetRules`, tested
/// with the core in `TabSetsFFITests`): the set a terminal is in. Keys and search stay inert.
struct SampleTabSetRules: TabSetRules {
    static let installed: Void = TabSets.install(SampleTabSetRules())

    func current(_ sets: [TabSet], containing terminal: Int) -> Int? { sets.firstIndex { $0.members.contains(terminal) } }
    func step(_ sets: [TabSet], from current: Int, forward: Bool) -> Int? { nil }
    func numbered(_ sets: [TabSet], _ number: Int) -> Int? { nil }
    func search(_ sets: [TabSet], _ query: String, name: (Int) -> String) -> [TabSets.Hit] { [] }
}
