// The type roles of the interface font at the theme's size (`gallery/typography.rs`): caption, label, body,
// heading and title, in the three text strengths.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

struct TypeSpecimen: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            line("Title", .title, tokens.textPrimary)
            line("Heading", .heading, tokens.textPrimary)
            line("Body text and list rows", .body, tokens.textPrimary)
            line("Label of a band", .label, tokens.textMuted)
            line("Caption, metadata and second lines", .caption, tokens.textMuted)
            line("Faint hints and placeholders", .caption, tokens.textFaint)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func line(_ text: String, _ role: TextRole, _ color: Color) -> some View {
        Text(text).font(role.font).foregroundStyle(color)
    }
}
