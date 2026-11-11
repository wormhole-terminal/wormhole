// Names the sidebar draws from: the coding agents Wormhole knows and the colours a group or folder can wear. Both
// are the Rust registries' own lists (`wormhole-term` `agents/mod.rs`, core `theme/folder_color`), so a
// terminal says the same in both apps.

import Foundation

public enum AgentID: String, Sendable, CaseIterable {
    case claude, codex, gemini, copilot, cursor, openCode, aider, amp, goose, qwen, crush, droid, auggie, kiro,
        cline, pi, fm

    /// The agent a command names (core's `Agent::command`, which `AgentInfo.key` carries).
    public init?(command: String) {
        guard let agent = Self.allCases.first(where: { $0.command == command }) else { return nil }
        self = agent
    }

    var command: String {
        switch self {
        case .cursor: "cursor-agent"
        case .openCode: "opencode"
        case .kiro: "kiro-cli"
        default: rawValue
        }
    }

    /// The agent's product name.
    public var name: String {
        switch self {
        case .claude: "Claude Code"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .codex: "Codex CLI"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .gemini: "Gemini CLI"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .copilot: "GitHub Copilot CLI"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .cursor: "Cursor Agent"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .openCode: "OpenCode"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .aider: "Aider"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .amp: "Amp"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .goose: "Goose"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .qwen: "Qwen Code"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .crush: "Crush"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .droid: "Factory Droid"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .auggie: "Auggie"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .kiro: "Kiro CLI"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .cline: "Cline CLI"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .pi: "Pi"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        case .fm: "Foundation Models"  // text: an agent's product name, English as in the agent registry (wormhole-term agents/mod.rs)
        }
    }

    /// The name without what kind of program it is ("Claude" for Claude Code), where a line names the
    /// agent at work.
    public var shortName: String {
        for kind in [" CLI", " Code", " Agent"] where name.hasSuffix(kind) {  // text: the kinds of program a product name ends with, matched against the names above
            return String(name.dropLast(kind.count))
        }
        return name
    }
}

/// A group's nine colour names, as core's `GroupColor` (`view/sidebar/group_color.rs`); how a theme draws
/// each is the design system's. Declared in the order the group's menu offers them.
public enum GroupColor: String, Sendable, CaseIterable {
    case grey, blue, red, yellow, green, pink, purple, cyan, orange

    /// The colours new groups take in turn, so two made one after the other don't look alike (grey is only
    /// ever chosen).
    public static let inTurn: [GroupColor] = [.blue, .green, .orange, .purple, .pink, .cyan, .yellow, .red]

    /// The colour a group saved before the nine names (Cyan was "teal", no grey) or with the American
    /// spelling means.
    public init?(key: String) {
        switch key {
        case "teal": self = .cyan
        case "gray": self = .grey
        default: self.init(rawValue: key)
        }
    }
}

/// A folder's eight colour names (core's `theme/folder_color`): a different set from a group's, with teal
/// where a group has cyan and no grey. For folders, not groups.
public enum FolderColor: String, Sendable, CaseIterable {
    case red, orange, yellow, green, teal, blue, purple, pink
}
