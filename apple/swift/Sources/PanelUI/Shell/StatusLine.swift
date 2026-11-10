// One caption line over a list (`problem.rs` `refresh_failed` and `compare_missing`): red when the last read failed under
// a list that is the last good one, amber when the compared branch is gone. A row tall, truncated at the end.

import DesignSystem
import SwiftUI

struct StatusLine: View {
    let text: String
    let danger: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(TextRole.caption.font).foregroundStyle(danger ? tokens.statusDangerText : tokens.statusWarningText)
            .lineLimit(1).truncationMode(.tail)
            .padding(.horizontal, Space._4).frame(maxWidth: .infinity, minHeight: Metric.row, alignment: .leading)
    }
}
