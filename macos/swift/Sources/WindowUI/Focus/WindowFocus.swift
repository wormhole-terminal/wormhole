// Where the keyboard is in one window and where F6, ⇥ and Esc take it (`window/focus` in the GPUI app). The rules are
// the core's (`Focus.rules`: F6's order, ⇥'s stops, the Esc ladder); this holds the real focus and the facts. The
// terminal and the two lists (the sidebar's, the panel's) keep the keyboard themselves; the rows of buttons, tabs and
// items are one `FocusHolder` view with a cursor each, as the GPUI app's handles are.

import AppKit
import Observation
import PanelUI
import SidebarUI
import TerminalMetal
import WorkspaceModel

@MainActor @Observable
public final class WindowFocus {
    /// What is on screen, written by the window's body: the columns, the status bar, terminals as tabs.
    struct Layout: Equatable {
        var sidebar = true
        var panel = false
        var statusBar = false
        var tabsOnTop = false
    }

    var layout = Layout()
    /// Which of the holder's areas the keys are on, and whether the holder has the keyboard now.
    private(set) var virtual = FocusArea.titleRow
    private(set) var holding = false
    var titleIndex = 0
    var statusIndex = 0
    /// The last input was a key and the window is in front: a ring shows (a click moves the focus without one).
    private(set) var keyboardDriven = false
    private(set) var windowKey = true
    /// A check's stand-in for the window being in front (`window-front on|off`; nil follows the window): every ring hides in a
    /// window that is not in front, so a shot can show it without taking the front from the app that has it.
    var frontOverride: Bool?
    /// A check's stand-in for macOS's Keyboard navigation (`keyboard-navigation on|off`; nil follows the system).
    public static var keyboardNavigationOverride: Bool?
    @ObservationIgnored unowned let hub: WindowHub
    @ObservationIgnored weak var holder: FocusHolderView?
    @ObservationIgnored private var observers: [any NSObjectProtocol] = []
    @ObservationIgnored private var monitor: Any?
    /// What had the keyboard before the first overlay (the palette, the shortcut sheet) opened.
    @ObservationIgnored private weak var beforeOverlay: NSResponder?

    init(hub: WindowHub) { self.hub = hub }

    // MARK: where the keys are

    /// The area that has the keyboard, if any.
    public func current() -> FocusArea? {
        switch holder?.window?.firstResponder {
        case is MetalTerminalView: return .terminal
        case is FocusHolderView: return virtual
        default: break
        }
        if hub.sidebarLink.hasKeyboard { return .sidebar }
        return hub.panel.keyboard.listFocused ? .panelContent : nil
    }

    /// Whether `area`'s ring shows, on its `index`th stop where it has several.
    func ring(_ area: FocusArea, _ index: Int = 0) -> Bool {
        holding && virtual == area && keyboardDriven && front && cursor(area) == index
    }

    /// Whether this window is in front as far as the rings go.
    public var front: Bool { frontOverride ?? windowKey }

    public func setFront(_ front: Bool?) { frontOverride = front }

    /// macOS's Keyboard navigation: ⇥ reaches the title row's buttons too.
    var keyboardNavigation: Bool { Self.keyboardNavigationOverride ?? NSApp.isFullKeyboardAccessEnabled }

    /// The stop an area's keys are on, for ⇥.
    private func cursor(of area: FocusArea) -> Int {
        switch area {
        case .titleRow: titleIndex
        case .panelContent: hub.panel.keyboard.stopAt
        default: 0
        }
    }

    func cursor(_ area: FocusArea) -> Int {
        switch area {
        case .titleRow: min(titleIndex, max(titleButtons - 1, 0))
        case .statusBar: min(statusIndex, max(hub.statusItems.count - 1, 0))
        default: 0
        }
    }

    /// The title row's buttons on screen: the sidebar's, and a new terminal's while the sidebar is open; none under tabs.
    var titleButtons: Int { layout.tabsOnTop ? 0 : layout.sidebar ? 2 : 1 }

    private var shown: Set<FocusArea> {
        var areas: Set<FocusArea> = [.titleRow, .terminal]
        if layout.sidebar { areas.insert(.sidebar) }
        // A diff or file covers the terminal and its title strip.
        if hub.panel.cover == nil { areas.insert(.tabs) }
        if layout.panel { areas.formUnion([.panelTabs, .panelContent]) }
        if layout.statusBar { areas.insert(.statusBar) }
        return areas
    }

    /// How many ⇥ stops an area has: none while hidden; the title row's buttons only with macOS's Keyboard navigation on.
    private func stops(of area: FocusArea) -> Int {
        guard shown.contains(area) else { return 0 }
        switch area {
        case .titleRow: return keyboardNavigation ? titleButtons : 0
        case .panelContent: return hub.panel.keyboard.stops
        default: return 1
        }
    }

    // MARK: moving

    /// F6 (1) and ⇧F6 (-1).
    public func step(_ delta: Int) {
        let to = Focus.rules.step(from: current(), delta: delta, shown: shown)
        // Back into the title row from behind, the last button is the nearest.
        focus(FocusStop(to, to == .titleRow && delta < 0 ? .max : 0))
    }

    /// ⇥ (1) and ⇧⇥ (-1).
    func tab(_ delta: Int) {
        let from = current().map { FocusStop($0, cursor(of: $0)) }
        let counts = Dictionary(uniqueKeysWithValues: FocusArea.allCases.map { ($0, stops(of: $0)) })
        focus(Focus.rules.tabStep(from: from, delta: delta, stops: counts))
    }

    func focus(_ stop: FocusStop) {
        keyboardDriven = true
        switch stop.area {
        case .terminal: return focusTerminal()
        case .sidebar: return swiftUI(hub.sidebarLink.requestKeyboard)
        case .panelContent where hub.panel.keyboard.lists > 0: return swiftUI { hub.panel.keyboard.request(stop: stop.index) }
        case .titleRow: titleIndex = min(stop.index, max(titleButtons - 1, 0))
        case .statusBar: statusIndex = min(statusIndex, max(hub.statusItems.count - 1, 0))
        case .tabs, .panelTabs, .panelContent: break
        }
        virtual = stop.area
        holding = true
        holder?.window?.makeFirstResponder(holder)
        announce()
    }

    /// ⇧⌘E: from anywhere to the terminal, and from the terminal to the notes while they are on (the note being worked
    /// on, else the first, else a new one, with the Notes tab and the panel shown: `window/shortcuts.rs`), else to the
    /// panel's list.
    public func toggleTerminalAndPanel() {
        if current() != .terminal { return focusTerminal() }
        if hub.panelTabs.contains(.notes) {
            hub.send(.showPanelTab(.notes))
            swiftUI { hub.panel.notes.focusIn() }
        } else if layout.panel {
            focus(FocusStop(.panelContent))
        }
    }

    /// A list of SwiftUI takes the keyboard when asked, but only from the window's own view: the holder or the terminal
    /// (AppKit views) give it back to that first.
    private func swiftUI(_ request: () -> Void) {
        if let window = holder?.window, window.firstResponder is FocusHolderView || window.firstResponder is MetalTerminalView {
            window.makeFirstResponder(window.contentView)
        }
        holding = false
        request()
    }

    // MARK: overlays

    /// An overlay opens or closes (`window/overlay.rs` `end_overlay`): the first one to open remembers who had the keyboard.
    func overlayChanging(to open: Bool) {
        if open, hub.state.overlay == nil { beforeOverlay = holder?.window?.firstResponder }
    }

    /// The last overlay closed (`close_overlay`): the keyboard returns to what had it, or to the terminal on screen when
    /// that is gone (a terminal closed meanwhile) or was nothing that takes keys.
    func overlayClosed() {
        guard hub.state.overlay == nil, let window = holder?.window else { return }
        let back = beforeOverlay as? NSView
        beforeOverlay = nil
        if let back, back.window === window, back !== window.contentView, window.makeFirstResponder(back) { return }
        focusTerminal()
    }

    /// The keyboard back to the terminal on screen: where it goes when a list, a strip or an editor lets go of it.
    public func focusTerminal() {
        guard let id = hub.state.selected, let view = hub.terminals.existing(id) else { return }
        view.window?.makeFirstResponder(view)
    }

    func holderChanged(holding now: Bool) {
        holding = now
        if now { announce() }
    }

    // MARK: Esc

    /// Esc, the nearest open thing first (`view::focus::escape`): a find bar, a diff over the terminal, then the
    /// terminal. `true` when it took the key; the terminal's Esc is its program's, and a text field's is its own.
    @discardableResult
    func escape() -> Bool {
        if hub.cancelColumnDrag?() == true || hub.drag.cancel() { return true }
        if hub.state.overlay != nil || holder?.window?.firstResponder is NSText { return false }
        let area = current()
        let find = hub.selectedOverlay?.find
        let detail = hub.panel.cover != nil || find?.isOpen == true
        switch Focus.rules.escape(terminal: area == .terminal, popup: false, detail: detail) {
        case .toShell, .closePopup:
            return false
        case .closeDetail:
            if let find, find.isOpen {
                find.close()
                focusTerminal()
            } else {
                hub.panel.closeCover()
                // The list keeps the keyboard: a second Esc goes to the terminal.
                if area != nil, area != .terminal { focus(FocusStop(.panelContent)) }
            }
            return true
        case .toTerminal:
            guard area != nil || holder?.window?.firstResponder == nil else { return false }
            focusTerminal()
            return true
        }
    }

    // MARK: the window's own events

    /// Hears keys and clicks in this window: a click ends the ring, and Esc over a terminal that has a find bar or a
    /// diff over it goes to the ladder before the program hears it.
    func attach(to holder: FocusHolderView) {
        guard monitor == nil, let window = holder.window else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .leftMouseDown, .rightMouseDown]) { [weak self] event in
            MainActor.assumeIsolated { self?.swallows(event, in: window) ?? false } ? nil : event
        }
        for name in [NSWindow.didBecomeKeyNotification, NSWindow.didResignKeyNotification] {
            observers.append(
                NotificationCenter.default.addObserver(forName: name, object: window, queue: .main) { [weak self] note in
                    let key = note.name == NSWindow.didBecomeKeyNotification
                    MainActor.assumeIsolated { self?.windowKey = key }
                })
        }
    }

    private func swallows(_ event: NSEvent, in window: NSWindow) -> Bool {
        guard event.window === window else { return false }
        keyboardDriven = event.type == .keyDown
        let bare = event.modifierFlags.intersection(.deviceIndependentFlagsMask).subtracting(.function).isEmpty
        // Esc closes the shortcut sheet whatever has the keyboard (`shortcut_sheet.rs`); its filter's own Esc clears first.
        if event.type == .keyDown, event.keyCode == Key.escape, bare, hub.state.overlay == .shortcuts, !(window.firstResponder is NSText) {
            hub.send(.showOverlay(nil))
            return true
        }
        return event.type == .keyDown && event.keyCode == Key.escape && bare && window.firstResponder is MetalTerminalView
            && escape()
    }

    func detach() {
        monitor.map(NSEvent.removeMonitor)
        monitor = nil
        observers.forEach(NotificationCenter.default.removeObserver)
        observers = []
    }
}
