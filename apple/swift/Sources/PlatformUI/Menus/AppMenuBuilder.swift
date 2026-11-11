// The full menu bar, driven by the keymap table: a `MenuProviding` (core's menus, or the dumped layout without the
// Rust core) makes the values for the window in front, the factory makes the native menus, and this installs them
// as `NSApp.mainMenu` (with the Window and Help menus registered so the system adds the window list and the Help
// search). It rebuilds only when what the menus show changed, and never while a menu is open.

import AppKit
import PlatformModel

@MainActor
public final class AppMenuBuilder {
    public let dispatcher: MenuDispatcher
    private let builder: any MenuProviding
    private let factory: NSMenuFactory
    private(set) var shown: MenuState?
    private var pending: MenuState?
    private var tracking = false
    // Read by `deinit`, which is not isolated; written once in `init`.
    private nonisolated(unsafe) var observers: [any NSObjectProtocol] = []

    public init(
        builder: any MenuProviding = MenuBuilder(), dispatcher: MenuDispatcher = MenuDispatcher(), style: MenuStyle = .themed(),
        standard: [MenuCommand: Selector] = StandardActions.selectors
    ) {
        self.builder = builder
        self.dispatcher = dispatcher
        factory = NSMenuFactory(dispatcher: dispatcher, style: style, standard: standard)
        let center = NotificationCenter.default
        observers = [
            center.addObserver(forName: NSMenu.didBeginTrackingNotification, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { self?.tracking = true }
            },
            center.addObserver(forName: NSMenu.didEndTrackingNotification, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated {
                    self?.tracking = false
                    if let state = self?.pending { self?.update(state) }
                }
            },
        ]
    }

    deinit { observers.forEach(NotificationCenter.default.removeObserver) }

    /// The bar for `state` as a native menu, not installed: for tests and offscreen dumps.
    public func mainMenu(for state: MenuState) -> NSMenu {
        let main = NSMenu(title: "Main")  // text: the menu bar's own name, never shown
        main.autoenablesItems = false
        for menu in builder.menus(for: state) {
            let holder = NSMenuItem(title: menu.title, action: nil, keyEquivalent: "")
            holder.submenu = factory.menu(title: menu.title, menu.items)
            main.addItem(holder)
        }
        return main
    }

    /// Puts the menus up for `state`; the same state costs nothing, and a menu that is open waits its turn.
    public func update(_ state: MenuState) {
        guard state != shown else { return }
        guard !tracking else { pending = state; return }
        pending = nil
        shown = state
        let main = mainMenu(for: state)
        NSApp.mainMenu = main
        NSApp.windowsMenu = main.items.first { $0.title == "Window" }?.submenu  // text: the title core gives that menu, found by it
        NSApp.helpMenu = main.items.first { $0.title == "Help" }?.submenu  // text: the title core gives that menu, found by it
    }
}
