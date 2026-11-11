// How the value is worked out when the theme doesn't set it: the rule from core, in the expression syntax of
// themes.md, and a line that says what typing a value does to it (`rule_section` of the GPUI app's editor).

import DesignSystem
import StudioModel
import SwiftUI

struct RuleSection: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let rule = document.rule(for: token)
        if rule != nil || token.hasPrefix("terminal.") {
            EditorSection {
                SectionLabel(StudioWords.sectionRule)
                if let rule {
                    Text(rule).codeFont().textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
                        .padding(Space._2).frame(maxWidth: .infinity, alignment: .leading)
                        .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                }
                Text(explanation(rule)).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private func explanation(_ rule: String?) -> String {
        guard let rule else { return StudioWords.ruleFromPalette }
        if document.file.own(token, in: document.shown) != nil { return StudioWords.ruleSetHere }
        let names = type(of: CoreKnowledge()).names(in: rule).filter { document.expressionNames.contains($0) }
        guard !names.isEmpty else { return StudioWords.ruleFixed }
        return StudioWords.ruleFollows(names.map { "`\($0)`" }.joined(separator: ", "))
    }
}
