// The keys of the core's keymap table that no menu item holds as its key equivalent: an action in no menu (Group 1 to 9,
// the intro and welcome trials) and the second key of one that is (⇧⌘N, ⇧⌘], ⇧⌘[, ⌘=). GPUI binds every key of every
// live entry; here a menu holds only an item's first key, so this one monitor resolves the rest from the same table
// (`Keymap.shared`) and runs the action through the router, as the menu would.

import AppKit
import PlatformModel

@MainActor
final class ExtraKeys {
    private var monitor: Any?
    private unowned let app: AppController

    init(app: AppController) { self.app = app }

    func start() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            MainActor.assumeIsolated { self?.runs(event) ?? false } ? nil : event
        }
    }

    /// True when the key was this table's to run.
    private func runs(_ event: NSEvent) -> Bool {
        // Only a window of the workspace: a sheet, Settings or the Studio read their own keys.
        guard !event.isARepeat, let window = event.window, let session = app.sessions.first(where: { $0.window === window }),
            let spec = Self.spec(of: event)
        else { return false }
        let keymap = Keymap.shared
        let inMenus = Self.menuActions(keymap.layout)
        guard let entry = keymap.entries.first(where: { $0.isLive && $0.keys.contains { $0.sameKey(spec) } }),
            entry.scope == .app, app.router.implemented.contains(entry.id)
        else { return false }
        // The first key of an action in a menu is that item's key equivalent: AppKit runs it, with its enabled state.
        if inMenus.contains(entry.id), entry.keys.first?.sameKey(spec) == true { return false }
        // What the menus hide (its feature is off in this window's profile) or grey, a key does not run either.
        let state = app.menuState(for: session.hub)
        guard !state.hidden.contains(entry.id), !state.disabled.contains(entry.id) else { return false }
        app.router.run(entry.id)
        return true
    }

    /// The key press as the table writes it: ⌘ or ⌃⌥ held (the keys a text field never takes), a one-character key.
    static func spec(of event: NSEvent) -> KeySpec? {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        var mods: KeyMods = []
        if flags.contains(.shift) { mods.insert(.shift) }
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.option) { mods.insert(.option) }
        guard mods.contains(.command) || mods.isSuperset(of: [.control, .option]),
            let key = event.charactersIgnoringModifiers, key.count == 1,
            key.unicodeScalars.first.map({ !(0xF700...0xF8FF).contains($0.value) }) == true
        else { return nil }
        return KeySpec(mods, key)
    }

    /// Every action some menu of the layout holds.
    private static func menuActions(_ layout: [MenuLayout]) -> Set<ActionID> {
        func ids(_ slots: [MenuLayout.Slot]) -> [ActionID] {
            slots.flatMap { slot -> [ActionID] in
                switch slot {
                case .action(let id): [id]
                case .submenu(_, let slots): ids(slots)
                default: []
                }
            }
        }
        return Set(layout.flatMap { ids($0.slots) })
    }
}
