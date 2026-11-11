// A card: 12 pt of padding, the large corner and a hairline edge (`style::card`). `edge` is the border colour;
// a card that needs attention passes the warning colour.

import DesignSystem
import SwiftUI

struct AuxCard<Content: View>: View {
    let edge: Color?
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        content
            .padding(Space._3)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay {
                RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
                    .strokeBorder(edge ?? tokens.border, lineWidth: tokens.metrics.borderWidth)
            }
            .marked("card")
    }
}
