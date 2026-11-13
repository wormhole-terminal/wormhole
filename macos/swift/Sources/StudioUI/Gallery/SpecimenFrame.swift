// A specimen's frame: its title and what it holds, and the content. The outline of the selected token is on the elements
// themselves (`.marked`), where GPUI puts it (`Env::mark`), not round the whole specimen.

import DesignSystem
import StudioModel
import SwiftUI

struct SpecimenFrame<Extras: View>: View {
    let specimen: Specimen
    let context: GalleryContext
    let extras: @MainActor (String) -> Extras
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                Text(specimen.title).textCase(.uppercase).font(TextRole.caption.font.weight(.semibold)).foregroundStyle(tokens.textMuted).fixedSize()
                Text(specimen.detail).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(1)
            }
            SpecimenContent.view(for: specimen.id, context: context, extras: extras).padding(Space._2)
        }
        .foregroundStyle(tokens.textPrimary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(specimen.title)
    }
}
