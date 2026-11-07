// The terminals found on this Mac and what each would bring over (`import/`).

import DesignSystem
import SettingsModel
import SwiftUI

struct ImportList: View {
    let records: [ImportRecord]
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(spacing: 0) {
            ForEach(records) { record in
                HStack {
                    VStack(alignment: .leading, spacing: 0) {
                        Text(record.app).font(.system(size: TextRole.body.size, weight: .medium)).foregroundStyle(tokens.textPrimary)
                        Text(record.summary).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                    }
                    Spacer()
                    Text(String(localized: "settings-import-review-soon", defaultValue: "Review…")).font(.system(size: TextRole.label.size))
                        .foregroundStyle(tokens.textFaint)
                }
                .frame(minHeight: SettingsMetric.rowMinHeight)
                .accessibilityElement(children: .combine)
            }
        }
    }
}
