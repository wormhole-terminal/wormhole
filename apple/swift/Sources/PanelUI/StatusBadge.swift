// The status letter of a changed file (M, A, D, R, ?). Colour backs the letter up; it never carries it alone.

import DesignSystem
import PanelModel
import SwiftUI

struct StatusBadge: View {
    let status: ChangeStatus

    var body: some View {
        Text(status.rawValue)
            .font(.system(size: 10, weight: .semibold, design: .monospaced))
            .foregroundStyle(PanelColors.status(status))
            .frame(width: 18, height: 18)
            .background(
                PanelColors.status(status).opacity(0.14),
                in: RoundedRectangle(cornerRadius: Radius.chip - 2, style: .continuous)
            )
            .accessibilityHidden(true)
    }
}

extension ChangeStatus {
    var spoken: String {
        switch self {
        case .modified: "Modified"
        case .added: "Added"
        case .deleted: "Deleted"
        case .renamed: "Renamed"
        case .untracked: "Untracked"
        }
    }
}
