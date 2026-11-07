// What an `NSMenuItem` does when chosen: it holds the item's `MenuCommand` and hands it to one closure, so a
// menu is only values and one callback (the window finds the terminal or group again by id: the list may have
// changed while the menu was open). Items that AppKit answers itself (Quit, Copy, Zoom) bypass this.

import AppKit
import PlatformModel

@MainActor
public final class MenuDispatcher: NSObject {
    /// Runs after the menu has closed, outside any menu tracking.
    public var handler: (MenuCommand) -> Void

    public init(handler: @escaping (MenuCommand) -> Void = { _ in }) { self.handler = handler }

    @objc func choose(_ sender: NSMenuItem) {
        guard let box = sender.representedObject as? CommandBox else { return }
        handler(box.command)
    }

    /// A row of swatches asks the same way, with the colour's index.
    func pick(_ command: MenuCommand) { handler(command) }
}

/// `representedObject` must be a class; the command is a value.
final class CommandBox: NSObject {
    let command: MenuCommand
    init(_ command: MenuCommand) { self.command = command }
}
