// The hotkey quick panel (`status_item/quick_panel/mod.rs`): a floating panel over whatever app is in front,
// without bringing Wormhole forward. It sits 20 % from the top of the screen the pointer is on, grows and shrinks
// with its rows keeping its top, and goes when it loses the keyboard (a click elsewhere) or on Esc. Pressed again
// while it is open, the hotkey shows Wormhole instead, so show and hide stays one more press away.

import AppKit
import DesignSystem
import Observation
import PlatformModel
import SwiftUI

@MainActor
public final class QuickPanelController: NSObject, NSWindowDelegate {
    public let model = QuickPanelModel()
    public private(set) var window: NSPanel?
    private var hosting: NSHostingView<AnyView>?
    private var clock: Task<Void, Never>?
    private let store: ThemeStore
    /// The terminals and Do items as they are now; read when the panel opens and once a second while it is open,
    /// so times and states stay current (never from inside a view's update).
    private let source: () -> (terminals: [AmbientTerminal], items: [QuickPanelDo])

    public var isOpen: Bool { window?.isVisible ?? false }

    public init(store: ThemeStore = .shared, source: @escaping () -> (terminals: [AmbientTerminal], items: [QuickPanelDo])) {
        self.store = store
        self.source = source
        super.init()
        model.close = { [weak self] in self?.close() }
    }

    /// The hotkey: opens the panel; pressed while it is open, closes it and runs `showApp`.
    public func toggle(showApp: () -> Void) {
        if isOpen {
            close()
            showApp()
        } else {
            open()
        }
    }

    public func open(on screen: NSScreen? = nil) {
        guard window == nil else { return }
        refresh()
        let size = NSSize(width: Metric.paletteWidth, height: model.height)
        let panel = QuickPanelWindow(contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.isReleasedWhenClosed = false
        panel.delegate = self
        panel.onCommandReturn = { [model] in model.commandReturn() }
        let host = NSHostingView(rootView: AnyView(root))
        host.frame = NSRect(origin: .zero, size: size)
        panel.contentView = host
        hosting = host
        window = panel
        let area = (screen ?? Self.pointerScreen).frame
        panel.setFrameTopLeftPoint(NSPoint(x: area.midX - size.width / 2, y: area.maxY - area.height * QuickPanelMetrics.topShare))
        panel.makeKeyAndOrderFront(nil)
        watch()
    }

    public func close() {
        clock?.cancel()
        clock = nil
        guard let panel = window else { return }
        window = nil
        hosting = nil
        // Deferred: this may run inside the panel's own update.
        DispatchQueue.main.async { panel.orderOut(nil); panel.close() }
    }

    public func windowDidResignKey(_ notification: Notification) { close() }

    private var root: some View { QuickPanelView(model: model).themed(store) }

    /// Reads the terminals again and keeps the window as tall as what it shows, its top where it is.
    func refresh() {
        (model.terminals, model.items) = source()
        model.now = Date()
        guard let panel = window else { return }
        let height = model.height
        var frame = panel.frame
        guard abs(frame.height - height) > 0.5 else { return }
        frame.origin.y += frame.height - height
        frame.size.height = height
        panel.setFrame(frame, display: true)
    }

    /// A one-second clock keeps times and states current; the window also follows its rows the moment they
    /// change (a query typed, Reply opened).
    private func watch() {
        clock = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(1))
                self?.refresh()
            }
        }
        followHeight()
    }

    private func followHeight() {
        guard window != nil else { return }
        withObservationTracking { _ = model.height } onChange: { [weak self] in
            Task { @MainActor in self?.refresh(); self?.followHeight() }
        }
    }

    /// The display the pointer is on (the panel opens there, as Spotlight does).
    private static var pointerScreen: NSScreen {
        NSScreen.screens.first { NSMouseInRect(NSEvent.mouseLocation, $0.frame, false) } ?? NSScreen.main ?? NSScreen.screens[0]
    }
}
