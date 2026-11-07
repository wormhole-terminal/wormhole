// Names the sidebar draws from: the coding agents Wormhole knows and the colours a folder can wear. Both
// are the Rust registries' own lists (`wormhole-term` `agents/mod.rs`, core `theme/folder_color`), so a
// terminal says the same in both apps.

import Foundation

public enum AgentID: String, Sendable, CaseIterable {
    case claude, codex, gemini, copilot, cursor, openCode, aider, amp, goose, qwen, crush, droid, auggie, kiro,
        cline, pi, fm

    /// The agent's product name.
    public var name: String {
        switch self {
        case .claude: "Claude Code"
        case .codex: "Codex CLI"
        case .gemini: "Gemini CLI"
        case .copilot: "GitHub Copilot CLI"
        case .cursor: "Cursor Agent"
        case .openCode: "OpenCode"
        case .aider: "Aider"
        case .amp: "Amp"
        case .goose: "Goose"
        case .qwen: "Qwen Code"
        case .crush: "Crush"
        case .droid: "Factory Droid"
        case .auggie: "Auggie"
        case .kiro: "Kiro CLI"
        case .cline: "Cline CLI"
        case .pi: "Pi"
        case .fm: "Foundation Models"
        }
    }

    /// The name without what kind of program it is ("Claude" for Claude Code), where a line names the
    /// agent at work.
    public var shortName: String {
        for kind in [" CLI", " Code", " Agent"] where name.hasSuffix(kind) {
            return String(name.dropLast(kind.count))
        }
        return name
    }
}

/// A folder's eight colour names; how a theme draws each is the design system's.
public enum FolderColor: String, Sendable, CaseIterable {
    case red, orange, yellow, green, teal, blue, purple, pink
}
