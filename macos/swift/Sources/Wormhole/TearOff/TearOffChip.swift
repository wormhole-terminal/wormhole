// The chip that follows a dragged terminal over the desktop (GPUI's `sidebar/tear_off.rs`): AppKit keeps sending the drag to the
// window it began in, which can draw only inside itself, so once the pointer is out the picture is a small transparent
// window of its own that lets the mouse through and follows the pointer; back inside, it is put away. Where it lies is
// worked out from the pointer's place in the window, so a scripted drag shows it where the script put the pointer.

import AppKit
import SidebarUI
import SwiftUI
import WorkspaceModel

@MainActor
final class TearOffChip {
    /// How far from the pointer the chip's card stands: to its right, a little above it (`GHOST_POINTER_GAP`, `_RISE`).
    static let gap: CGFloat = 12
    static let rise: CGFloat = 14

    private var panel: NSPanel?
    private var host: NSHostingView<TearOffChipView>?
    /// The card's size as drawn (without the room for its shadow): what the new window grows from.
    private(set) var cardSize = CGSize(width: 160, height: 28)

    /// The chip over the screen at the global point `pointer` (AppKit's space, y up), or put away.
    func show(_ chip: TearOffChipView, at pointer: CGPoint, touchesSystem: Bool) {
        let host = host ?? NSHostingView(rootView: chip)
        host.rootView = chip
        let size = host.fittingSize
        let bleed = TearOffChipView.margin
        cardSize = CGSize(width: size.width - 2 * bleed, height: size.height - 2 * bleed)
        let panel = panel ?? makePanel(host)
        // The card's top left at the pointer's right and above; the window has the shadow's room around it.
        let corner = CGPoint(x: pointer.x + Self.gap, y: pointer.y + Self.rise)
        panel.setFrame(NSRect(x: corner.x - bleed, y: corner.y - size.height + bleed, width: size.width, height: size.height), display: true)
        if panel.isVisible { return }
        if touchesSystem { panel.orderFrontRegardless() } else { panel.orderBack(nil) }
    }

    func hide() { panel?.orderOut(nil) }

    /// The drag is over: the chip goes.
    func end() {
        panel?.orderOut(nil)
        panel = nil
        host = nil
    }

    private func makePanel(_ host: NSHostingView<TearOffChipView>) -> NSPanel {
        let panel = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.title = "Tear-off chip" // text: the title of a borderless panel, never drawn
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.level = .floating
        panel.isReleasedWhenClosed = false
        panel.collectionBehavior = [.transient, .ignoresCycle, .fullScreenAuxiliary]
        host.sizingOptions = []
        panel.contentView = host
        (self.panel, self.host) = (panel, host)
        return panel
    }
}
