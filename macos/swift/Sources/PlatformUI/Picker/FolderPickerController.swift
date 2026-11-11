// Opens the folder picker over a window (`picker/mod.rs`, `platform/macos/picker.rs`): a native panel against the
// terminal's cursor, above its line and inside the pane (`PickerPlacement`), re-placed as rows come and go without
// ever crossing the line. It closes on Esc, on a pick, and when it stops being the key window (a click elsewhere).
// A test instance never shows it: `--background` leaves the window ordering and the focus alone.

import AppKit
import DesignSystem
import PlatformModel
import SwiftUI

@MainActor
public final class FolderPickerController: NSObject, NSWindowDelegate {
    public private(set) var driver: FolderPickerDriver?
    public private(set) var panel: NSPanel?
    private let store: ThemeStore
    private let policy: PlatformPolicy
    private weak var parent: NSWindow?
    private var pane = CGRect.zero, cursor = CGRect.zero

    public var isOpen: Bool { panel != nil }

    public init(store: ThemeStore = .shared, policy: PlatformPolicy = .current()) {
        self.store = store
        self.policy = policy
    }

    /// `pane` and `cursor` are in the window's content space, origin top left. `onPick` gets what ↩ hands over.
    public func open(
        over parent: NSWindow, pane: CGRect, cursor: CGRect, context: PickerContext, providers: [any FolderProviding],
        decorators: [any FolderDecorating] = [], onPick: @escaping (String) -> Void, onForget: @escaping (String) -> Void = { _ in }
    ) {
        close()
        self.parent = parent
        (self.pane, self.cursor) = (pane, cursor)
        let driver = FolderPickerDriver(context: context, providers: providers, decorators: decorators)
        driver.onPick = { [weak self] path in self?.close(); onPick(path) }
        driver.onForget = onForget
        driver.onClose = { [weak self] in self?.close() }
        driver.onChange = { [weak self] in self?.place() }
        self.driver = driver
        let window = FolderPickerWindow(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        window.isOpaque = false
        window.backgroundColor = .clear
        window.hasShadow = true
        window.isReleasedWhenClosed = false
        window.delegate = self
        let host = NSHostingView(rootView: FolderPickerView(driver: driver).themed(store))
        host.sizingOptions = []
        window.contentView = host
        panel = window
        place()
        guard policy.touchesSystem else { return driver.open() }
        parent.addChildWindow(window, ordered: .above)
        window.makeKeyAndOrderFront(nil)
        driver.open()
    }

    public func close() {
        driver?.close()
        guard let panel else { return }
        self.panel = nil
        driver = nil
        parent?.removeChildWindow(panel)
        panel.orderOut(nil)
    }

    public func windowDidResignKey(_ notification: Notification) { close() }

    /// Places the card for the rows it has now (its side is decided by whether it fits at its largest, so typing
    /// never moves it across the cursor's line).
    func place() {
        guard let panel, let parent, let driver else { return }
        let card = FolderPickerMetrics.card(hasCrumbs: !driver.model.answer.crumbs.isEmpty)
        let placed = PickerPlacement.place(pane: pane, cursor: cursor, card: card, wanted: FolderPickerView.rowCount(driver.model))
        // The window's content in screen space: the top-left origin of the placement is its top edge.
        let content = parent.contentRect(forFrameRect: parent.frame)
        let frame = NSRect(
            x: content.minX + placed.rect.minX, y: content.maxY - placed.rect.maxY, width: placed.rect.width, height: placed.rect.height)
        if panel.frame != frame { panel.setFrame(frame, display: true) }
    }
}
