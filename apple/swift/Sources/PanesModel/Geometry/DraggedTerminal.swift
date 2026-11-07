// What a terminal being dragged carries, so a pane's header, a sidebar row and the pane area's drop agree: it
// travels as plain text (`wormhole.terminal:<id>:<origin>`), which every drag session can carry.

import Foundation

public struct DraggedTerminal: Sendable, Equatable {
    /// Where the drag started: a sidebar row joins a pane to the split, a pane header moves it within it.
    public enum Origin: String, Sendable { case sidebarRow, paneHeader }

    public var id: PaneID
    public var origin: Origin

    public init(id: PaneID, origin: Origin) { (self.id, self.origin) = (id, origin) }

    private static let prefix = "wormhole.terminal"

    public var encoded: String { "\(Self.prefix):\(id):\(origin.rawValue)" }

    public init?(encoded: String) {
        let parts = encoded.split(separator: ":").map(String.init)
        guard parts.count == 3, parts[0] == Self.prefix, let id = Int(parts[1]), let origin = Origin(rawValue: parts[2]) else {
            return nil
        }
        self.init(id: id, origin: origin)
    }
}
