// The two split buttons at the row's right end: Split right (⌘D) and Split down (⇧⌘D), each greyed out
// where the focused pane has no room for a half. Folded away in a narrow row, they live in the ⋯ menu.

import DesignSystem
import PanesModel
import SwiftUI

struct SplitButtons: View {
    let actions: TitleActions

    var body: some View {
        HStack(spacing: 0) {
            TitleIconButton(label: PanesText.splitRightKey, enabled: actions.canSplit(.row), motion: .newPane(.right), action: { actions.split(.row) }) {
                LucideView(icon: .columns2, size: Metric.titleGlyph)
            }
            TitleIconButton(label: PanesText.splitDownKey, enabled: actions.canSplit(.column), motion: .newPane(.below), action: { actions.split(.column) }) {
                LucideView(icon: .rows2, size: Metric.titleGlyph)
            }
        }
    }
}
