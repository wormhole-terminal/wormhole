// In place of a git tab's list when git could not read the folder at all (`render_git_problem`): the headline as an empty
// note, the hint under it, the command or first line in a well in the code font, and Retry and Copy error as outlined
// buttons. Everything starts at the panel's 16 pt inset, like the rows it stands in for.

import DesignSystem
import PanelModel
import SwiftUI

struct GitProblemView: View {
    let problem: GitProblem
    let retry: () -> Void
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont
    @Environment(\.panelCopy) private var copy
    /// `problem.rs` `BUTTON_H`: a text button in a panel is 24 tall.
    private static let buttonHeight: CGFloat = 24

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            EmptyNote(text: problem.headline)
            if let hint = problem.hint {
                Text(hint).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).padding(.horizontal, Space._3)
            }
            Text(problem.detail).font(codeFont.font(TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                .textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading).padding(Space._2)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .padding(.horizontal, Space._3)
            HStack(spacing: Space._2) {
                outlined(panelText("git-retry", "Retry"), action: retry)
                if let message = problem.copy { outlined(panelText("git-copy-error", "Copy error")) { copy(message) } }
            }
            .padding(.horizontal, Space._3).padding(.bottom, Space._2)
        }
    }

    private func outlined(_ title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).font(TextRole.caption.font).foregroundStyle(tokens.buttonText).padding(.horizontal, Space._2)
                .frame(height: Self.buttonHeight)
                .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}
