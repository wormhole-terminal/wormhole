// A page's group (`style::setting_group`): its name above as a quiet label (13 / 600, muted) with a note at the
// right, an optional paragraph, then the rows in one rounded card (`settings.group` fill, 16 pt at the sides)
// with a divider between each two. Increase Contrast, where a faint fill is no edge, gives the card its outline.

import DesignSystem
import SwiftUI

struct SettingGroup<Rows: View, Accessory: View>: View {
    let title: String
    var aside = ""
    var blurb = ""
    /// At the head's right end, after the note: a results group's link to its page.
    @ViewBuilder var accessory: Accessory
    @ViewBuilder let rows: Rows
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            if !title.isEmpty || !aside.isEmpty {
                HStack(alignment: .lastTextBaseline) {
                    Text(title).textRole(.label).foregroundStyle(tokens.textMuted)
                    Spacer(minLength: Space._4)
                    Text(aside).textRole(.caption).foregroundStyle(tokens.textFaint).lineLimit(1)
                    accessory
                }
                .padding(.horizontal, Space._4)
                .accessibilityAddTraits(.isHeader)
            }
            if !blurb.isEmpty {
                Text(blurb).textRole(.caption).foregroundStyle(tokens.textMuted)
                    .padding(.horizontal, Space._4)
            }
            // A hairline between the rows a builder produced, so a row never has to know its neighbours.
            VStack(spacing: 0) {
                Group(subviews: rows) { rows in
                    ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                        if index > 0 { Rectangle().fill(tokens.settingsDivider).frame(height: tokens.metrics.borderWidth) }
                        row
                    }
                }
            }
                .padding(.horizontal, Space._4).padding(.vertical, Space._1)
                .background(tokens.settingsGroup, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
                .contrastOutline(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg), \.cardEdge)
        }
    }
}

extension SettingGroup where Accessory == EmptyView {
    init(title: String, aside: String = "", blurb: String = "", @ViewBuilder rows: () -> Rows) {
        self.init(title: title, aside: aside, blurb: blurb, accessory: { EmptyView() }, rows: rows)
    }
}
