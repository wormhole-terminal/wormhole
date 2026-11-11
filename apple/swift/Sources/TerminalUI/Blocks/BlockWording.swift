// How a command's end is worded: the readout beside a hovered block ("exit 101 · 3.5s"), the sentence VoiceOver
// says when ⌘↑ lands on it, and the gutter mark's colour (`blocks.rs`, `wormhole-term` `took_text`/`readout_text`,
// `view::spoken::command_block`). VoiceOver's durations come from Foundation, in the person's language.

import Foundation

enum BlockWording {
    /// "0.4s", "3.5s", "12s", "1m 5s", "2h 3m".
    static func took(_ took: Duration) -> String {
        let seconds = Int(took.components.seconds)
        switch seconds {
        case ..<10: return String(format: "%.1fs", Double(seconds) + Double(took.components.attoseconds) / 1e18)
        case ..<60: return "\(seconds)s"
        case ..<3600: return "\(seconds / 60)m \(seconds % 60)s"
        default: return "\(seconds / 3600)h \(seconds % 3600 / 60)m"
        }
    }

    /// "exit 101 · 3.5s", "3.5s" when the shell gave no code; nil for a command that has not finished.
    static func readout(_ state: BlockState) -> String? {
        guard case let .finished(code, took) = state else { return nil }
        let parts = [code.map { L10n.blockExit($0) }, took.map(Self.took)].compactMap { $0 }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    /// What VoiceOver says for the block ⌘↑ / ⌘↓ landed on.
    static func spoken(_ block: CommandBlock) -> String {
        let command = block.command.split(whereSeparator: \.isNewline).first.map { $0.trimmingCharacters(in: .whitespaces) } ?? ""
        var parts = command.isEmpty ? [] : [command]
        switch block.state {
        case .prompt: parts.append(L10n.blockSpokenPrompt)
        case .running: parts.append(L10n.blockSpokenRunning)
        case let .finished(code, took):
            switch code {
            case 0?: parts.append(L10n.blockSpokenFinished)
            case let code?: parts.append(L10n.blockSpokenFailed(code))
            case nil where took != nil: parts.append(L10n.blockSpokenFinished)
            case nil where command.isEmpty: parts.append(L10n.blockSpokenEmpty)
            case nil: break
            }
            if let took { parts.append(took.formatted(.units(allowed: [.hours, .minutes, .seconds], width: .wide, fractionalPart: .hide(rounded: .down)))) }
        }
        return parts.joined(separator: ", ")
    }
}
