// Every block on the grid, for the control folder's `blocks` and `block <n>` (GPUI's `devctl_blocks`): the ids come
// from the engine's prompt marks, walked up from the bottom, so the list is the engine's, not a copy kept here.

import Foundation
import TerminalCore

extension BlockController {
    /// The blocks on the grid, oldest first.
    public func all() -> [CommandBlock] {
        var found: [CommandBlock] = []
        var line = source.bottommostLine + 1
        // A cap, not a rule: a grid keeps far fewer prompts than this.
        for _ in 0..<100_000 {
            guard let prompt = source.promptLine(from: line, direction: .up) else { break }
            if let block = source.block(atLine: prompt), found.last?.id != block.id { found.append(block) }
            line = prompt
        }
        return found.reversed()
    }

    /// One line per block, as GPUI writes them: `n: lines a..=b[ output c..=d] | state | command`.
    public func listing() -> String {
        all().enumerated().map { number, block in
            let state: String = switch block.state {
            case .prompt: "prompt"
            case .running: "running"
            case .finished: BlockWording.readout(block.state) ?? "nothing ran"
            }
            let output = block.outputLines.map { " output \($0.lowerBound)..=\($0.upperBound)" } ?? ""
            return "\(number + 1): lines \(block.start)..=\(block.end)\(output) | \(state) | \(block.command.replacingOccurrences(of: "\n", with: "⏎"))"
        }.joined(separator: "\n")
    }
}
