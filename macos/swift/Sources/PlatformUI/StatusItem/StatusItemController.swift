// The menu bar extra (`platform/macos/status_item`): one `NSStatusItem` whose glyph says whether a terminal needs
// input and whose menu lists the terminals by status. The menu is built as it opens (`menuNeedsUpdate`), so every
// row's time is right when it is read. The working animation is one one-shot timer per frame, and only while
// working is the state; Reduce Motion freezes it on the full prompt. A test instance (`--background`) has no
// item: nothing of it shows in the menu bar.

import AppKit
import PlatformModel

@MainActor
public final class StatusItemController: NSObject, NSMenuDelegate {
    /// Four frames, a quarter second each (`notify::MENU_ANIM_FRAMES`, `MENU_ANIM_FRAME`).
    static let frames = 4
    static let frameDuration: Duration = .milliseconds(250)

    public let dispatcher: MenuDispatcher
    public private(set) var item: NSStatusItem?
    public private(set) var model: StatusMenuModel?
    private let style: MenuStyle
    private let keymap: Keymap
    private let now: () -> Date
    private var snapshot = StatusSnapshot()
    private var animation: Task<Void, Never>?

    public init(
        policy: PlatformPolicy = .current(), style: MenuStyle = .themed(), keymap: Keymap = .shared,
        dispatcher: MenuDispatcher = MenuDispatcher(), now: @escaping () -> Date = Date.init
    ) {
        self.style = style
        self.keymap = keymap
        self.dispatcher = dispatcher
        self.now = now
        super.init()
        guard policy.touchesSystem else { return }
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        item.button?.setAccessibilityLabel("Wormhole") // text: the item's accessibility name, the product name
        item.menu = NSMenu(title: "Wormhole") // text: the item's accessibility name, the product name
        item.menu?.delegate = self
        self.item = item
    }

    deinit { animation?.cancel() }

    /// Draws the glyph for `snapshot` and keeps it for the menu that opens next.
    public func update(_ snapshot: StatusSnapshot) {
        self.snapshot = snapshot
        let model = StatusMenu.build(snapshot, now: now(), keymap: keymap)
        self.model = model
        show(model.icon, frame: nil)
        animate(model.icon.animate)
    }

    public func menuNeedsUpdate(_ menu: NSMenu) {
        let model = StatusMenu.build(snapshot, now: now(), keymap: keymap)
        NSMenuFactory(dispatcher: dispatcher, style: style).fill(menu, with: model.menu)
    }

    private func show(_ icon: StatusIcon, frame: Int?) {
        guard let button = item?.button else { return }
        button.image = StatusIconImage.image(
            filled: icon.state == .working, frame: frame, mark: icon.mark, colors: .init(style: style), description: icon.tooltip)
        button.imagePosition = icon.count == nil ? .imageOnly : .imageLeft
        button.title = icon.count ?? ""
        button.toolTip = icon.tooltip
    }

    private func animate(_ on: Bool) {
        animation?.cancel()
        animation = nil
        guard on, !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion, let icon = model?.icon else { return }
        animation = Task { [weak self] in
            var frame = 0
            while !Task.isCancelled {
                try? await Task.sleep(for: Self.frameDuration)
                frame = (frame + 1) % Self.frames
                self?.show(icon, frame: frame)
            }
        }
    }
}
