// Every syntax colour the theme styles, each in its own colour (bold and italic as the theme sets them) on the
// terminal's background, where code is drawn; a click selects the token (`gallery/syntax.rs`).

import DesignSystem
import SwiftUI

struct CodeSpecimen: View {
    let context: GalleryContext
    @Environment(\.tokens) private var tokens

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 130), alignment: .leading)], alignment: .leading, spacing: Space._1) {
            ForEach(tokens.syntaxKeys, id: \.self) { key in
                let style = tokens.syntaxStyle(key)
                Button { context.select("syntax.\(key)") } label: {
                    Text(key).font(TextRole.caption.font.monospaced().weight(style?.bold == true ? .bold : .regular).italic(style?.italic == true))
                        .foregroundStyle(style?.color.color ?? tokens.terminal.foreground.color)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .buttonStyle(.plain).accessibilityLabel("syntax.\(key)")
            }
        }
        .padding(Space._3).frame(maxWidth: .infinity, alignment: .leading)
        .background(tokens.terminal.background.color, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .marked("code")
    }
}
