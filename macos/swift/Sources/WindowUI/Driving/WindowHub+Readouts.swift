// text-file: the control folder's `status-items`, `tab-bar`, `tab-click` and `switcher` (dev tooling), not interface text
// What the window's status bar and tab bar hold, as text, from the same values `WindowBody.statusBar` and
// `TerminalColumn` hand their views (`StatusSummary`, `StatusBarDetail`, `tabBar`, `tabActions`). Nothing here decides. The
// shape is `DesignSystem.Readout`'s:
//
//     status-items detail="words" width="1600" shown
//     profile "Work"
//     attention 7 "api" words="needs input · 2m" tooltip="api, then web · go to the first (⌃⌘↓)" spoken="…"
//     count working 2 first="4" words="2 working" spoken="2 terminals working"
//     state "working · 4m"
//     hosting "Persistent"
//     size "vim · 100×30"
//     probe "…"

import DesignSystem
import Foundation
import PanesModel
import PanesUI
import PlatformModel
import TerminalUI
import WordsModel
import WorkspaceModel

extension WindowHub {
    /// `status-items`: the bar's items in its order, at the window's width (`width`, in points), whether or not the bar is on.
    public func statusItems(look: WindowLook, width: CGFloat) -> String {
        let everyone = actions.everyTerminal?() ?? state.all
        let summary = StatusSummary(everyone)
        let detail = StatusSummary.Detail(windowWidth: width)
        let selected = state.selectedTerminal
        var lines = [
            Readout.line(
                "status-items", pairs: [("detail", "\(detail)"), ("width", String(Int(width)))], flags: [look.statusBar ? "shown" : "hidden"])
        ]
        if let profile = actions.windowProfile?(), detail.words { lines.append(Readout.line("profile", quoted: [profile.name])) }
        if let waiting = summary.attention {
            let prefix = detail.words ? Words.text("window-state-needs-input", fallback: "needs input") : ""
            let words = waiting.since.flatMap { store.waited(since: $0) }.map { (prefix.isEmpty ? "" : prefix + " · ") + $0.words } ?? prefix
            var pairs = [("words", words), ("tooltip", summary.attentionTooltip(jumpKey: Keymap.shared.key(.goToWaiting)?.glyphText) ?? "")]
            if let since = waiting.since, let spoken = store.spoken(attention: waiting.title, since: since) { pairs.append(("spoken", spoken)) }
            lines.append(Readout.line("attention", [String(waiting.id)], quoted: [waiting.title], pairs: pairs))
        }
        for count in summary.rest {
            var pairs = [("first", String(count.first)), ("words", detail.words ? count.words : String(count.number))]
            if let spoken = store.spoken(count: count.number, of: count.activity) { pairs.append(("spoken", spoken)) }
            lines.append(Readout.line("count", ["\(count.activity)", String(count.number)], pairs: pairs))
        }
        if detail.size { for item in look.probeItems { lines.append(Readout.line("probe", quoted: [item.text], pairs: [("tooltip", item.tooltip)])) } }
        if let selected, let words = selected.activity.stateWords { lines.append(Readout.line("state", quoted: [stateWords(selected, words)])) }
        if let hosting = state.selected.flatMap({ store.hosting(of: $0) }), let said = StatusHosting.words(hosting) {
            lines.append(Readout.line("hosting", quoted: [said.word], pairs: [("tooltip", said.sentence)]))
        }
        // The grid the program was told, as `WindowBody.statusBar` reads it.
        if let selected, detail.size, let grid = state.selected.flatMap({ terminals.existing($0) })?.gridSize {
            lines.append(Readout.line("size", quoted: ["\(store.programName(of: selected.id) ?? selected.title) · \(grid.cols)×\(grid.rows)"]))
        }
        return lines.joined(separator: "\n")
    }

    /// "working · 4m", "failed · exit 1", "needs input": `StatusBar.stateText`'s words.
    private func stateWords(_ terminal: TerminalSummary, _ words: String) -> String {
        if terminal.activity == .failed, let code = terminal.failureCode { return StateWords.failedExit(code) }
        if [.working, .finished, .failed].contains(terminal.activity), let since = terminal.since, let waited = store.waited(since: since) {
            return words + " · " + waited.words
        }
        return words
    }

    /// `tab-bar`: the bar the window shows over the terminal while the terminals are tabs.
    public func tabBarListing(look: WindowLook) -> String {
        TabBarReadout.bar(tabBar) + "\n" + Readout.line("tabs", flags: [look.tabsOnTop ? "on-top" : "in-sidebar"])
    }

    /// `switcher [query]`: what the group switcher's popover lists.
    public func switcherListing(look: WindowLook, query: String) -> String {
        TabBarReadout.switcher(tabBar, actions: tabActions(appearance: look.sidebar), query: query)
    }

    /// `tab-click <n|+>`: the n-th tab (from 1) clicked, or the bar's +, as the bar's own actions do. Nil when done, else why not.
    public func tabClick(_ target: String, look: WindowLook) -> String? {
        let bar = tabBar
        let actions = tabActions(appearance: look.sidebar)
        if target == "+" {
            actions.newTerminal(TabBarReadout.currentGroup(bar))
            return nil
        }
        guard let n = Int(target), bar.tabs.indices.contains(n - 1) else { return "no tab \(target) of \(bar.tabs.count)" }
        actions.select(bar.tabs[n - 1].id)
        return nil
    }
}

extension WindowHub {
    /// `title-menu`: the selected terminal's title row menu as items (`TitleRowMenu`'s), nil without a terminal.
    public var titleMenuItems: [TitleMenuReadout.Item?]? { selectedTitleActions.map { TitleMenuReadout.items(row: titleRow, actions: $0) } }
}

extension WindowHub {
    /// `switcher open`: the tab bar opens its group switcher, as a click on its button does (the bar must show it).
    public func openSwitcher() { switcherAsk += 1 }

    /// `switcher-press new-group|sidebar|card <n>|add <n>`: the switcher's footer buttons and a card (`GroupSwitcherView`'s
    /// actions: New group, Show all groups in the sidebar, a card's click, a pinned empty card's +). Nil when done.
    public func switcherPress(_ argument: String, look: WindowLook) -> String? {
        let words = argument.split(separator: " ").map(String.init)
        let bar = tabBar
        let actions = tabActions(appearance: look.sidebar)
        let index = words.count > 1 ? Int(words[1]) : nil
        switch words.first {
        case "new-group": actions.newGroup()
        case "sidebar": actions.showGroupsInSidebar()
        case "card":
            guard let index, bar.cards.indices.contains(index) else { return "no card \(words.dropFirst().joined())" }
            actions.openSet(index)
        case "add":
            guard let index, bar.sets.indices.contains(index), let group = bar.sets[index].groupID, bar.cards[index].pinned, bar.cards[index].count == 0
            else { return "card \(words.dropFirst().joined()) has no +" }
            actions.newTerminal(group)
        default: return "switcher-press new-group|sidebar|card <n>|add <n>"
        }
        return nil
    }
}

extension WindowHub {
    /// The selected terminal's find bar (`find-state`).
    public var selectedFind: FindModel? { selectedOverlay?.find }
}
