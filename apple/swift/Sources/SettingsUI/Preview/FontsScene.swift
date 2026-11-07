// Fonts' scene (`draw_specimen`): a prompt, a result and code with the pairs ligatures join, set in the terminal
// font at its size and line height on the theme's terminal colours.

import DesignSystem
import SettingsModel
import SwiftUI

struct FontsScene: View {
    let model: SettingsModel

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let size = model.value("font_size").number ?? 14
        let height = model.value("line_height").number ?? 1.3
        let font = PreviewFont.mono(model.value("font_family").string ?? "Menlo", size: size)
        VStack(alignment: .leading, spacing: max(0, size * (height - 1.2))) {
            Text("$ cargo test --release").foregroundStyle(t.terminal.foreground.color)
            Text("test result: ok. 412 passed").foregroundStyle(t.terminal.ansi[2].color)
            Text("fn map(x: &str) -> Option<u8> {").foregroundStyle(t.terminal.ansi[4].color)
            Text("    x.parse().ok() // != == => -> >=").foregroundStyle(t.terminal.foreground.color)
            Text("}").foregroundStyle(t.terminal.ansi[4].color)
        }
        .font(font)
        .padding(Space._4)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(t.terminal.background.color, in: RoundedRectangle(cornerRadius: t.metrics.radius))
    }
}
