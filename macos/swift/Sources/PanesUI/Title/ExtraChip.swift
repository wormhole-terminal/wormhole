// A chip in the title row: an optional icon and a short text, 20 tall. A filled one sits on the neutral
// `state.active` fill with an outline and reads in the text colour (a port, a pull request); without it only
// the muted text is left (the "+2" after the ports). `band.hover` (8 %) vanishes on the title row in several
// themes and over blur, which is why the chip takes the stronger fill (`style::extra_chip`).

import DesignSystem
import PanesModel
import SwiftUI

struct ExtraChip: View {
    let text: String
    var symbol: (name: String, colour: Color)?
    var filled = true
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        HStack(spacing: TitleMetrics.extraGap) {
            if let symbol {
                Image(systemName: symbol.name).font(.system(size: TitleMetrics.extraIcon - 2, weight: .medium))
                    .foregroundStyle(symbol.colour).frame(width: TitleMetrics.extraIcon)
            }
            Text(text).lineLimit(1).fixedSize()
        }
        .font(tokens.font(.caption).weight(.medium))
        .foregroundStyle(filled ? tokens.textPrimary : tokens.textMuted)
        .padding(.horizontal, TitleMetrics.extraPad)
        .frame(height: TitleMetrics.chipHeight)
        .background { if filled { shape.fill(tokens.stateActive) } }
        .overlay { if filled { shape.strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) } }
        .contentShape(shape)
        .marked("neutral.chip")
    }
}
