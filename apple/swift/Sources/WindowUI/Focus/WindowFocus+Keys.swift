// The keys of the areas the holder stands for (`window/focus/bars.rs`): ← → along the buttons, tabs and items, wrapping;
// ↩ and Space press the one the keys are on; the terminal's strip steps between terminals and the panel's tabs open as
// the keys reach them. ⇥ and Esc are the same for every area (`tab`, `escape`).

import AppKit
import SwiftUI
import TerminalMetal
import WorkspaceModel

/// The key codes the window answers (Carbon's `kVK_*`, which AppKit hands out as `NSEvent.keyCode`).
enum Key {
    static let tab: UInt16 = 48, escape: UInt16 = 53, space: UInt16 = 49, returnKey: UInt16 = 36, enter: UInt16 = 76
    static let left: UInt16 = 123, right: UInt16 = 124, down: UInt16 = 125
}

extension WindowFocus {
    /// A key while the holder has the keyboard. `false` leaves it to AppKit (the other chords, ⌘-keys are the menu's).
    func holderKey(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask).subtracting(.function)
        switch event.keyCode {
        case Key.escape where flags.isEmpty: return escape()
        case Key.tab where flags.subtracting(.shift).isEmpty:
            tab(flags.contains(.shift) ? -1 : 1)
            return true
        case Key.left where flags.isEmpty: return along(-1)
        case Key.right where flags.isEmpty: return along(1)
        case Key.down where flags.isEmpty && virtual == .panelTabs: return press()
        case Key.returnKey where flags.isEmpty, Key.enter where flags.isEmpty, Key.space where flags.isEmpty: return press()
        default: return false
        }
    }

    /// ⇥ and Esc while a list or a field of SwiftUI has the keyboard: the same ladder and stops.
    func listKey(_ press: KeyPress) -> KeyPress.Result {
        // A field keeps its own ⇥ and Esc, and the terminal's belong to its program (the hosting view hears them first).
        if holder?.window?.firstResponder is NSText || holder?.window?.firstResponder is MetalTerminalView { return .ignored }
        switch press.key {
        case .tab where press.modifiers.subtracting(.shift).isEmpty:
            tab(press.modifiers.contains(.shift) ? -1 : 1)
            return .handled
        case .escape where press.modifiers.isEmpty: return escape() ? .handled : .ignored
        default: return .ignored
        }
    }

    private func along(_ delta: Int) -> Bool {
        func wrap(_ at: Int, _ count: Int) -> Int { count == 0 ? 0 : (at + delta + count) % count }
        switch virtual {
        case .titleRow: titleIndex = wrap(cursor(.titleRow), titleButtons)
        case .statusBar: statusIndex = wrap(cursor(.statusBar), hub.statusItems.count)
        case .tabs: walkTerminals(delta)
        case .panelTabs: walkPanelTabs(delta)
        default: return false
        }
        announce()
        return true
    }

    private func press() -> Bool {
        switch virtual {
        case .titleRow: hub.send(cursor(.titleRow) == 0 ? .toggleSidebar : .newTerminal)
        case .statusBar: hub.statusItems.indices.contains(cursor(.statusBar)) ? hub.send(.select(hub.statusItems[cursor(.statusBar)])) : ()
        case .tabs: focusTerminal()
        case .panelTabs: focus(FocusStop(.panelContent))
        default: return false
        }
        return true
    }

    /// ← → on the strip: the previous or next terminal the sidebar lists, wrapping; the keys stay on the strip.
    private func walkTerminals(_ delta: Int) {
        let ids = hub.state.sections.flatMap(\.rows).map(\.id)
        guard let at = ids.firstIndex(where: { $0 == hub.state.selected }), ids.count > 1 else { return }
        hub.send(.select(ids[(at + delta + ids.count) % ids.count]))
    }

    private func walkPanelTabs(_ delta: Int) {
        let tabs = hub.panelTabs
        guard let at = tabs.firstIndex(of: hub.state.panelTab), tabs.count > 1 else { return }
        hub.send(.showPanelTab(tabs[(at + delta + tabs.count) % tabs.count]))
    }

    /// What VoiceOver says where the keys are now, and that they moved.
    func announce() {
        let words: String =
            switch virtual {
            case .titleRow:
                "Title row, \(cursor(.titleRow) == 0 ? (hub.state.sidebarVisible ? "Hide sidebar" : "Show sidebar") : "New terminal"), button \(cursor(.titleRow) + 1) of \(titleButtons)"
            case .tabs: "Terminal, \(hub.state.selectedTerminal?.title ?? "none")"
            case .panelTabs: "Right panel tabs, \(hub.state.panelTab.rawValue.capitalized)"
            case .statusBar: "Status bar, item \(cursor(.statusBar) + 1) of \(hub.statusItems.count)"
            default: "Right panel"
            }
        holder?.speak(words)
    }
}
