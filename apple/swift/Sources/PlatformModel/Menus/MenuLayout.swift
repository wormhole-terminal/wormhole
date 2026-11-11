// The menu bar's layout as core writes it (`keymap/menus.rs` slots): which actions each menu holds, in what
// groups, and where the parts that follow the window in front go.

import Foundation

public struct MenuLayout: Sendable, Decodable {
    public var title: String
    /// `debug`: the menu is there only in a debug build (Developer).
    public var when: String?
    public var slots: [Slot]

    public indirect enum Slot: Sendable, Decodable {
        case action(ActionID)
        case separator
        case services
        case submenu(title: String, slots: [Slot])
        case dynamic(Dynamic)

        private enum CodingKeys: String, CodingKey { case kind, id, title, slots, name }

        public init(from decoder: any Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            switch try c.decode(String.self, forKey: .kind) {
            case "action": self = .action(try c.decode(ActionID.self, forKey: .id))
            case "separator": self = .separator
            case "services": self = .services
            case "submenu": self = .submenu(title: try c.decode(String.self, forKey: .title), slots: try c.decode([Slot].self, forKey: .slots))
            case "dynamic": self = .dynamic(try c.decode(Dynamic.self, forKey: .name))
            case let other: throw DecodingError.dataCorruptedError(forKey: .kind, in: c, debugDescription: "slot \(other)")  // text: a decoding error, not interface text
            }
        }
    }

    /// The parts a window fills in.
    public enum Dynamic: String, Sendable, Decodable {
        case recentFolders, projectActions, terminals, newTerminalWith, profiles
    }
}
