// The panel's few colours. System colours, not hex: they follow light, dark and Increase Contrast. Tints are
// the same hue at low opacity, so a diff stays readable behind code in both appearances.

import PanelModel
import SwiftUI

enum PanelColors {
    static let added = Color(nsColor: .systemGreen)
    static let removed = Color(nsColor: .systemRed)

    static let addedTint = added.opacity(0.13)
    static let removedTint = removed.opacity(0.13)
    static let hunkHeader = Color.primary.opacity(0.05)

    static func status(_ status: ChangeStatus) -> Color {
        switch status {
        case .modified: Color(nsColor: .systemOrange)
        case .added: added
        case .deleted: removed
        case .renamed: Color(nsColor: .systemBlue)
        case .untracked: Color.secondary
        }
    }
}
