// A release's notes: Markdown as the changelog writes it (`## Added`, `- a line`, paragraphs, inline code and
// links). Block structure is drawn here; each line's inline Markdown is the system's `AttributedString`.

import AuxModel
import DesignSystem
import SwiftUI

struct ReleaseNotesView: View {
    let notes: String
    @Environment(\.tokens) private var tokens

    private enum Block: Hashable {
        case heading(String), bullet(String), paragraph(String)
    }

    private var blocks: [Block] {
        notes.split(separator: "\n", omittingEmptySubsequences: true).map { line in
            let text = line.trimmingCharacters(in: .whitespaces)
            if text.hasPrefix("#") { return .heading(text.drop { $0 == "#" }.trimmingCharacters(in: .whitespaces)) }
            if text.hasPrefix("- ") || text.hasPrefix("* ") { return .bullet(String(text.dropFirst(2))) }
            return .paragraph(text)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in
                switch block {
                case .heading(let text):
                    Text(text).font(TextRole.label.font).padding(.top, Space._2).accessibilityAddTraits(.isHeader)
                case .bullet(let text):
                    HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                        Text("\u{2022}").foregroundStyle(tokens.textMuted).accessibilityHidden(true)
                        inline(text)
                    }
                case .paragraph(let text): inline(text)
                }
            }
        }
        .font(TextRole.body.font)
        .frame(maxWidth: .infinity, alignment: .leading)
        .textSelection(.enabled)
    }

    private func inline(_ text: String) -> Text {
        Text((try? AttributedString(markdown: text)) ?? AttributedString(text))
    }
}
