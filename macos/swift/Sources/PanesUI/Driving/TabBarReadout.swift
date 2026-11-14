// text-file: the control folder's `tab-bar` and `switcher` (dev tooling), not interface text
// The tab bar and its group switcher as text: what `TabBarView`, `GroupSwitcherView` and `SetCardView` draw, read from the
// model and actions the window hands them. Nothing here decides. The shape is `DesignSystem.Readout`'s:
//
//     tab-bar shown switcher set="Build" alert="none"
//     tab 4 "api" activity="working" number="2" alert="none" selected
//     plus "New terminal"
//
//     switcher query="" placeholder="Go to group or terminal…" open
//     card 0 "Build" count="3" tiles="7,4,9" activities="needsInput,working,idle" alert="attention" current pinned add="New terminal in Build" spoken="Build, 3 terminals, current"
//     empty 1 "No terminals"
//     footer "New group" "Show all groups in the sidebar" keys="⌃⌥[ / ] previous, next group" range="⌃⌥1–9"
//     hit terminal 7 "api" note="Build" mark="tile"
//     hit set 0 "Build" note="group · 3 terminals" mark="dot"
//     none "No matches"

import DesignSystem
import PanesModel
import WorkspaceModel

@MainActor
public enum TabBarReadout {
    /// `tab-bar`: the bar, then a line a tab in its order and the +.
    public static func bar(_ model: TabBarModel) -> String {
        var lines = [
            Readout.line(
                "tab-bar", pairs: [("set", model.setLabel), ("alert", "\(model.alertKind)")],
                flags: [model.isShown ? "shown" : "hidden", Readout.flag("switcher", model.showsSwitcher)])
        ]
        for tab in model.tabs {
            var pairs = [("activity", "\(tab.activity)"), ("alert", "\(tab.alert ?? TabAlert.none)")]
            if let number = tab.number { pairs.append(("number", String(number))) }
            if let spoken = tab.spoken { pairs.append(("spoken", spoken)) }
            lines.append(Readout.line("tab", [String(tab.id)], quoted: [tab.name], pairs: pairs, flags: [Readout.flag("selected", tab.id == model.selected)]))
        }
        if model.isShown { lines.append(Readout.line("plus", quoted: [PanesText.newTerminal])) }
        return lines.joined(separator: "\n")
    }

    /// `switcher [query]`: the popover's cards and footer, or with a query what its search lists; `open` while it is on screen.
    public static func switcher(_ model: TabBarModel, actions: TabBarActions, query: String, open: Bool) -> String {
        var lines = [
            Readout.line(
                "switcher", pairs: [("query", query), ("placeholder", PanesText.switcherSearch)],
                flags: [Readout.flag("hidden", !model.showsSwitcher), Readout.flag("open", open)])
        ]
        if query.trimmingCharacters(in: .whitespaces).isEmpty {
            for card in model.cards {
                let set = model.sets[card.id]
                // `GroupSwitcherView`: a pinned group with nothing in it has a + to start a terminal there.
                let add = card.pinned && card.count == 0 && set.groupID != nil
                var pairs = [
                    ("count", String(card.count)), ("tiles", actions.loudest(set.members).prefix(8).map(String.init).joined(separator: ",")),
                    ("activities", card.activities.map { "\($0)" }.joined(separator: ",")), ("alert", "\(card.alertKind)"),
                    ("spoken", PanesText.cardLabel(card.label, count: card.count, current: card.current, alert: card.alert)),
                ]
                if add { pairs.append(("add", PanesText.newTerminalIn(card.label))) }
                lines.append(
                    Readout.line(
                        "card", [String(card.id)], quoted: [card.label], pairs: pairs,
                        flags: [Readout.flag("current", card.current), Readout.flag("pinned", card.pinned), Readout.flag("group", set.groupID != nil)]))
                if card.count == 0, card.pinned { lines.append(Readout.under("empty", [String(card.id)], quoted: [PanesText.noTerminals])) }
            }
            let keys = actions.groupKeys()
            lines.append(Readout.line("footer", quoted: [PanesText.newGroup, PanesText.showGroupsInSidebar], pairs: [("keys", keys.hint), ("range", keys.keys)]))
            return lines.joined(separator: "\n")
        }
        let hits = TabSets.search(model.sets, query) { model.names[$0] ?? "" }
        if hits.isEmpty { lines.append(Readout.line("none", quoted: [PanesText.noMatches])) }
        for hit in hits {
            switch hit {
            case .set(let at):
                lines.append(
                    Readout.line(
                        "hit", ["set", String(at)], quoted: [model.sets[at].label],
                        pairs: [("note", PanesText.groupNote(count: model.sets[at].members.count)), ("mark", "dot")]))
            case .terminal(let id, let at):
                lines.append(
                    Readout.line(
                        "hit", ["terminal", String(id)], quoted: [model.names[id] ?? ""],
                        pairs: [("note", model.sets[at].label), ("mark", actions.tile == nil ? "none" : "tile")]))
            }
        }
        return lines.joined(separator: "\n")
    }
}
