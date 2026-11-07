// F6's areas (`view/focus`): the search field, the nav's list and the page, in that order. F6 moves forward,
// ⇧F6 back; Esc clears a search first and closes the window when there is nothing to clear.

enum SettingsFocus: Hashable, CaseIterable {
    case search, nav, content

    func next(backwards: Bool) -> SettingsFocus {
        let all = Self.allCases
        let index = all.firstIndex(of: self) ?? 0
        return all[(index + (backwards ? all.count - 1 : 1)) % all.count]
    }
}
