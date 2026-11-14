// What the window gives the sidebar (its hooks), and the terminals as tabs when the person shows them there instead of in the
// sidebar. A group's own edits (rename, pin, ungroup, close) are the core's actions, sent by the sidebar itself.

import AppKit
import DesignSystem
import PanesModel
import PanesUI
import PlatformModel
import SidebarUI
import SwiftUI
import WordsModel
import WorkspaceModel

extension WindowHub {
    /// What the list asks of the window: the close questions, the keyboard going between the list and the terminal, and a
    /// folder's copy (with the "Copied" notice) and reveal.
    var sidebarHooks: SidebarHooks {
        var hooks = SidebarHooks()
        hooks.closeTerminal = { [unowned self] in actions.closeTerminal($0) }
        hooks.closeGroup = { [unowned self] in actions.closeGroup($0) }
        hooks.focusTerminal = { [unowned self] in focus.focusTerminal() }
        hooks.browse = { [unowned self] in terminals.keepKeys() }
        hooks.copyPath = { [unowned self] in copyFolder($0) }
        hooks.revealFolder = { [unowned self] in revealFolder($0) }
        hooks.groupMenu = actions.groupMenu
        return hooks
    }

    /// "Copy path": the folder is copied and the chip says so (as the title row's does).
    public func copyFolder(_ path: String) {
        Pasteboard.setText(path)
        notices.post(.copied(.path))
    }

    /// "Reveal in Finder" (a test instance reveals nothing: `launchesApps`).
    public func revealFolder(_ path: String) {
        guard launchesApps else { return }
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
    }

    /// The tab bar for the terminals as they are: the groups as sets, the loose terminals as the last one, as the core lists
    /// them (`view::sidebar::tabs`), with the current set and the alerts it decides.
    var tabBar: TabBarModel {
        let state = state
        let facts = store.tabSets()
        // Without the core (the sample) the window lists them itself: the groups' terminals from `all` (a folded group lists
        // no rows in the sidebar, and the bar has no folds), then the loose ones.
        let sets = facts.map { facts in
            facts.sets.map { TabSet(groupID: $0.group, label: $0.group == nil ? TabSets.ungrouped : $0.label, members: $0.members) }
        } ?? TabSets.of(
            groups: state.groups.map { head in
                TabSet(groupID: head.id, label: head.name, members: state.all.filter { $0.group == head.id }.map(\.id))
            },
            loose: state.all.filter { $0.group == nil }.map(\.id))
        let current = facts.map { $0.current ?? 0 } ?? state.selected.flatMap { TabSets.current(sets, containing: $0) } ?? 0
        // A tab's number is the core's ⌘ number (stable per terminal; a terminal folded away has none).
        let numbers = Dictionary(state.sections.flatMap(\.rows).compactMap { row in row.number.map { (row.id, $0) } }, uniquingKeysWith: { first, _ in first })
        var bar = TabBarModel.make(
            sets: sets, current: current, selected: state.selected,
            terminal: { id in
                let terminal = state.all.first { $0.id == id }
                var item = TabItem(id: id, name: terminal?.title ?? "", activity: terminal?.activity ?? .idle, number: numbers[id])
                // Decided by the tile's urgency, as the row shows it, not by the activity.
                item.alert = terminal.map(TerminalStatusTile.alert(of:))
                return item
            },
            pinned: { set in state.groups.first { $0.id == set.groupID }?.isPinned ?? false },
            alerts: facts.map { facts in (sets: facts.sets.map { TabAlert($0.alert) }, others: TabAlert(facts.othersAlert)) })
        // What VoiceOver hears for each tab is the core's (`spoken::tab_label`): name, state, selection, "n of m".
        if let spoken = store.tabLabels(of: bar.tabs.map(\.id)), spoken.count == bar.tabs.count {
            for at in bar.tabs.indices where !spoken[at].isEmpty { bar.tabs[at].spoken = spoken[at] }
        }
        return bar
    }

    /// The switcher's footer from the keymap (`switcher.rs`): "⌃⌥[ / ] previous, next group" and "⌃⌥1–9".
    private static var groupKeyHints: (hint: String, keys: String) {
        let keymap = Keymap.shared
        guard let previous = keymap.key("previousGroup")?.glyphText, let next = keymap.key("nextGroup")?.glyphText,
              let first = keymap.key(.group(1))?.glyphText else { return ("", "") }
        let nextKey = next.last.map(String.init) ?? ""
        let hint = Words.text(
            "tab-switcher-keys-hint", ["previous": .text(previous), "next": .text(nextKey)], fallback: "\(previous) / \(nextKey) previous, next group")
        return (hint, "\(first)–9")
    }

    /// "New terminal with ▸" under a right-click of a + (the title row's, the tab bar's).
    var newTerminalMenu: AnyView {
        AnyView(ShellsMenu(shells: actions.shells()) { [unowned self] in actions.newTerminalWith($0) })
    }

    /// What the bar's controls do (the one-line forwards of `TabBarActions`), with the menus of a tab and of the +.
    func tabActions(appearance: SidebarAppearance) -> TabBarActions {
        var bar = TabBarActions(
            // A click on a tab gives that terminal the keyboard (`Activate`, `sidebar_host.rs`), as a click on the row does.
            select: { [unowned self] in
                send(.select($0))
                focus.focusTerminal()
            },
            close: { [unowned self] in actions.closeTerminal($0) },
            newTerminal: { [unowned self] group in
                if let group { newTerminal(inGroup: group) } else { send(.newTerminal) }
            },
            openSet: { [unowned self] in openSet(at: $0) },
            newGroup: { [unowned self] in send(.newTerminalInNewGroup) },
            showGroupsInSidebar: { [unowned self] in actions.showTerminalsInSidebar() },
            tabMenu: { [unowned self] id in
                AnyView(TerminalContextMenu(
                    id: id, state: state, appearance: appearance, hooks: sidebarHooks, link: sidebarLink, planner: store, send: send))
            },
            newMenu: { [unowned self] in newTerminalMenu },
            groupKeys: { Self.groupKeyHints })
        bar.groupMenu = actions.groupMenu
        // A tab's and a card's tiles are the rows' own (the agent's logo, the status light and badge).
        bar.tile = { [unowned self] id, size in
            state.all.first { $0.id == id }.map { TerminalStatusTile(terminal: $0, size: size, appearance: appearance) }
        }
        bar.loudest = { [unowned self] ids in
            TerminalStatusTile.loudestFirst(ids.compactMap { id in state.all.first { $0.id == id } }).map(\.id)
        }
        bar.cardWash = { [unowned self] group, tokens in state.groups.first { $0.id == group }?.cardWash(tokens)?.color }
        if case .rename(.group(let group))? = sidebarLink.request { bar.renameAsk = (group, sidebarLink.requests) }
        bar.switcherAsk = switcherAsk
        bar.groupPopover = { [unowned self] group, close in
            AnyView(GroupEditPopover(
                group: group, state: state, appearance: appearance, hooks: sidebarHooks, link: sidebarLink, planner: store, send: send,
                close: close))
        }
        return bar
    }

    /// Open a tab set: the terminal last selected in it, else its first; an empty group gets a new terminal
    /// (`tab_bar.rs` `open_set`).
    func openSet(at index: Int) {
        let sets = tabBar.sets
        guard sets.indices.contains(index) else { return }
        let set = sets[index]
        if let target = store.lastSelected(in: set.groupID).flatMap { set.members.contains($0) ? $0 : nil } ?? set.members.first {
            send(.select(target))
        } else if let group = set.groupID {
            newTerminal(inGroup: group)
        }
    }

    /// A terminal in `group` from the tab bar's + or an empty set opened (also the keys' Ctrl-Opt-N): the group's folder is
    /// asked about first, as the sidebar's + does (`group_folder.rs` `new_session_in_group`: every way a terminal is made in a group).
    func newTerminal(inGroup group: GroupID) {
        let name = state.groups.first { $0.id == group }?.name ?? ""
        Task { @MainActor in
            await GroupFolderQuestion.openTerminal(inGroup: group, name: name, planner: store, send: send)
        }
    }
}
