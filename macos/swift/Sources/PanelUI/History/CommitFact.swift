// A labelled fact of the opened commit (`details_view.rs` `fact_row`): the label in a 72 pt column of muted text,
// the value after it, a button at the end.

import DesignSystem
import SwiftUI

struct CommitFact<Value: View, Trailing: View>: View {
    let label: String
    @ViewBuilder let value: Value
    @ViewBuilder var trailing: Trailing
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(alignment: .top, spacing: PanelMetrics.gap) {
            Text(label).foregroundStyle(tokens.textMuted).frame(width: 72, alignment: .leading)
            value.foregroundStyle(tokens.textPrimary).frame(maxWidth: .infinity, alignment: .leading)
            trailing
        }
    }
}

extension CommitFact where Trailing == EmptyView {
    init(label: String, @ViewBuilder value: () -> Value) {
        self.init(label: label, value: value, trailing: { EmptyView() })
    }
}
