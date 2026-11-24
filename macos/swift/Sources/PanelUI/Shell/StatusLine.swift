// One caption line over a list (`problem.rs` `refresh_failed` and `compare_missing`): red when the last read failed under
// a list that is the last good one, amber when the compared branch is gone. At least a row tall: an error is a sentence, so it wraps.

import DesignSystem
import SwiftUI

struct StatusLine: View {
    let text: String
    let danger: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(tokens.font(.caption)).foregroundStyle(danger ? tokens.statusDangerText : tokens.statusWarningText)
            .padding(.horizontal, Space._4).padding(.vertical, Space._1).frame(maxWidth: .infinity, minHeight: Metric.row, alignment: .leading)
    }
}
