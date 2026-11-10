// A specimen's frame: its title and what it holds, the content, and a dashed outline in the primary while the
// selected token is drawn on it, so an edit shows where it lands (`uses_outline`, `Env::mark`).

import DesignSystem
import StudioModel
import SwiftUI

struct SpecimenFrame: View {
    let specimen: Specimen
    let context: GalleryContext
    @Environment(\.tokens) private var tokens

    var body: some View {
        let outlined = context.selected.map { specimen.use.draws(with: $0) } ?? false
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                Text(specimen.title).textCase(.uppercase).font(TextRole.caption.font.weight(.semibold)).foregroundStyle(tokens.textMuted).fixedSize()
                Text(specimen.detail).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(1)
            }
            SpecimenContent.view(for: specimen.id, context: context)
                .padding(Space._2)
                .overlay {
                    RoundedRectangle(cornerRadius: tokens.metrics.radius)
                        .strokeBorder(tokens.primary, style: StrokeStyle(lineWidth: max(2, tokens.metrics.focusRingWidth), dash: [4, 3]))
                        .opacity(outlined ? 1 : 0).allowsHitTesting(false)
                        .motion(Motion.press, value: outlined)
                }
        }
        .foregroundStyle(tokens.textPrimary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(specimen.title)
        .accessibilityValue(outlined ? "uses the selected token" : "")
    }
}
