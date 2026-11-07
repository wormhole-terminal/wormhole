// What a terminal's tile says and how loud it is, as the core words it (`workspace::TileGlyph`, `view::sidebar::Urgency`):
// a folded group's head draws one tile per member from these, and a row's tile follows its glyph.

import Foundation

/// The mark on a tile (core `TileGlyph`).
public enum TileGlyph: Sendable, Equatable {
    case working, needsYou, done, failed, idle
    /// A plain terminal; `unread` is the dot for output nobody has seen.
    case terminal(unread: Bool)
    case shell
}

/// How loud a terminal is, the loudest first (core `Urgency`): the order a folded head lists its tiles in.
public enum Urgency: Int, Sendable, Equatable, Comparable {
    case needsYou, failed, working, bell, newOutput, running, done, idle

    public static func < (a: Self, b: Self) -> Bool { a.rawValue < b.rawValue }
}

/// One terminal of a folded group, drawn as a tile on its head (core `MemberTile`).
public struct MemberTile: Sendable, Equatable, Identifiable {
    public var id: TerminalID
    public var name: String
    public var glyph: TileGlyph
    public var activity: Activity
    public var agent: AgentID?
    public var urgency: Urgency

    public init(
        id: TerminalID, name: String, glyph: TileGlyph = .shell, activity: Activity = .idle, agent: AgentID? = nil,
        urgency: Urgency = .idle
    ) {
        (self.id, self.name, self.glyph, self.activity, self.agent, self.urgency) = (id, name, glyph, activity, agent, urgency)
    }
}
