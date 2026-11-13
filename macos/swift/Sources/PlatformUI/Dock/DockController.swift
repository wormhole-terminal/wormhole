// The Dock icon (`platform/dock.rs`): its badge (how many terminals need input), one informational bounce when a
// terminal starts needing you while Wormhole is in the background, and its menu (`applicationDockMenu`). An
// unchanged badge costs nothing. A test instance never touches the real Dock: what would have been applied is
// read from `badge` and `bounces`, and its icon is dropped (an accessory app has none and takes no place in ⌘⇥).

import AppKit
import PlatformModel
import WordsModel

@MainActor
public final class DockController {
    /// What was last applied, so an unchanged badge costs nothing (and tests can read it).
    public private(set) var badge: String?
    public private(set) var bounces = 0
    public let dispatcher: MenuDispatcher
    private let policy: PlatformPolicy
    private let style: MenuStyle
    private var menu: [MenuNode] = []

    public init(policy: PlatformPolicy = .current(), style: MenuStyle = .themed(), dispatcher: MenuDispatcher = MenuDispatcher()) {
        self.policy = policy
        self.style = style
        self.dispatcher = dispatcher
    }

    /// The red badge on the icon; nil clears it. The system draws it (it isn't themed).
    public func setBadge(_ label: String?) {
        guard label != badge else { return }
        badge = label
        if policy.touchesSystem { NSApp.dockTile.badgeLabel = label }
    }

    /// One informational bounce, never the critical one that keeps going.
    public func bounce() {
        bounces += 1
        if policy.touchesSystem { NSApp.requestUserAttention(.informationalRequest) }
    }

    /// Whether Wormhole is the active app (a window stays the main window while another app is in front).
    public var appIsActive: Bool { NSApp.isActive }

    /// A test instance has no Dock icon and no place in ⌘⇥.
    public func hideIconIfBackground() {
        guard !policy.touchesSystem else { return }
        if !NSApp.setActivationPolicy(.accessory) { AppLog.log("error", "background mode: could not drop the Dock icon") }
    }

    /// Puts the badge and the menu up for the terminals as they are now.
    public func update(terminals: [AmbientTerminal], badgeMode: DockBadgeMode, now: Date = Date()) {
        let counts = AmbientCounts(of: terminals)
        setBadge(badgeMode.label(for: counts))
        menu = DockMenu.build(terminals, now: now)
    }

    /// What `applicationDockMenu(_:)` returns: macOS adds its own items below.
    public func dockMenu() -> NSMenu { NSMenuFactory(dispatcher: dispatcher, style: style).menu(title: "", menu) }
}
