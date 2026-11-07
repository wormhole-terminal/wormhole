// The bar along the bottom of a pane whose shell has ended (`render/exited.rs`): what ended and how (or that its
// session host went away with it), a new shell in the same place, or closing the terminal. 38 pt tall (a title
// row), on the panel surface with a rule above.

import DesignSystem
import SwiftUI

public struct ExitedBar: View {
    public let shell: String
    public let exitCode: Int?
    /// The session host went away with the program.
    public let hostLost: Bool
    let actions: PaneBarActions
    /// The Close terminal action's keys, as the keymap shows them.
    let closeKeys: String
    @Environment(\.tokens) private var tokens
    @State private var closeHover = false

    public init(shell: String, exitCode: Int?, hostLost: Bool = false, closeKeys: String = "⌘W", actions: PaneBarActions) {
        (self.shell, self.exitCode, self.hostLost, self.closeKeys, self.actions) = (shell, exitCode, hostLost, closeKeys, actions)
    }

    /// What the bar says first (`exited_words`).
    public var words: String {
        if hostLost { return L10n.exitedLost }
        return exitCode.map { L10n.exitedWithCode(shell, $0) } ?? L10n.exited(shell)
    }

    public var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "terminal").font(.system(size: Metric.glyph)).foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            Text(words).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                .lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
            BarButton(title: L10n.restart, symbol: "arrow.clockwise", action: actions.restart)
            Button(action: actions.closeTerminal) {
                HStack(spacing: Space._1) {
                    Text(L10n.closeTerminal).font(.system(size: TextRole.caption.size))
                    KeyCapLabel(text: closeKeys)
                }
                .foregroundStyle(closeHover ? tokens.textPrimary : tokens.textMuted)
                .padding(.horizontal, Space._2)
                .frame(height: Metric.control)
                .background(closeHover ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .onHover { closeHover = $0 }
            .accessibilityLabel(L10n.closeTerminal)
        }
        .padding(.leading, Space._4).padding(.trailing, Space._2)
        .frame(height: Metric.titleRow)
        .background(tokens.surfacePanel)
        .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(words)
    }
}
