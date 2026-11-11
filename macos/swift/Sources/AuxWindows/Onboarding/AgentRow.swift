// An agent on PATH, as the third step lists it: a monogram, its name and where its hooks go, and either an
// Install (or Repair) button, a badge saying they are in, or a badge saying its status is read from its output.
// Only the button installs anything (`agents_step.rs`).

import AuxModel
import DesignSystem
import SwiftUI

struct AgentRow: View {
    let agent: FoundAgent
    let install: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            Monogram(name: agent.name)
            VStack(alignment: .leading, spacing: 0) {
                Text(agent.name).font(TextRole.body.font.weight(.medium))
                HStack(spacing: Space._1) {
                    Text(agent.hooks == .installed ? AuxText.hooksIn : agent.hooks == .unsupported ? AuxText.onPathOutput : AuxText.onPath + " \u{B7}")
                    if agent.hooks != .unsupported, let path = agent.path {
                        Text(path).font(.system(size: TextRole.caption.size, design: .monospaced)).lineLimit(1).truncationMode(.middle)
                            .padding(.horizontal, Space._1)
                            .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                    }
                }
                .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            }
            Spacer(minLength: 0)
            switch agent.hooks {
            case .installed:
                HStack(spacing: Space._1) { StatusBadge(text: AuxText.installed, color: tokens.statusSuccessText); Image(systemName: "checkmark").foregroundStyle(tokens.statusSuccessText) }
            case .unsupported:
                StatusBadge(text: AuxText.readingOutput, color: tokens.textMuted)
            case .partial:
                Button(AuxText.repair, action: install).secondaryAction().controlSize(.small)
            case .notInstalled:
                Button(AuxText.install, action: install).secondaryAction().controlSize(.small)
            }
        }
        .padding(.horizontal, Space._6).padding(.vertical, Space._2)
        .accessibilityElement(children: .contain)
    }
}

/// The agent's first letter on a tile: the brand tiles of the GPUI app come from the agent registry, which is
/// Rust's; until the FFI gives them, a neutral tile that does not pretend to be a logo.
struct Monogram: View {
    let name: String
    var dimmed = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(String(name.prefix(1)))
            .font(TextRole.label.font).foregroundStyle(dimmed ? tokens.textFaint : tokens.tileMark)
            .frame(width: Metric.tile, height: Metric.tile)
            .background(tokens.tileBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.tileEdge, lineWidth: tokens.metrics.borderWidth) }
            .accessibilityHidden(true)
    }
}
