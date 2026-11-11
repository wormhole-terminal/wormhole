// A context menu under the pointer or under the row the keys are on (⌃↩): the same native `NSMenu` as the menu
// bar's, from the same values. `onClose` runs once the menu has closed, whether an item was chosen or not, so a
// row's focus ring can go (`Menu::on_close`).

import AppKit
import PlatformModel

@MainActor
public final class ContextMenuPresenter {
    public let dispatcher: MenuDispatcher
    private let style: MenuStyle

    public init(dispatcher: MenuDispatcher = MenuDispatcher(), style: MenuStyle = .themed()) {
        self.dispatcher = dispatcher
        self.style = style
    }

    public func menu(_ nodes: [MenuNode]) -> NSMenu {
        NSMenuFactory(dispatcher: dispatcher, style: style).menu(title: "", nodes)
    }

    /// Opens the menu at `point` in `view`'s space (the view's top-left origin when it is flipped).
    public func show(_ nodes: [MenuNode], at point: NSPoint, in view: NSView?, onClose: (() -> Void)? = nil) {
        guard !nodes.isEmpty else { return }
        menu(nodes).popUp(positioning: nil, at: point, in: view)
        onClose?()
    }
}
