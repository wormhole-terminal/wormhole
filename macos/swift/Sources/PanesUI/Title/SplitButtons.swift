// The two split buttons at the row's right end: Split right (⌘D) and Split down (⇧⌘D), each greyed out
// where the focused pane has no room for a half. Folded away in a narrow row, they live in the ⋯ menu.

import DesignSystem
import PanesModel
import SwiftUI

struct SplitButtons: View {
    let actions: TitleActions

    var body: some View {
        HStack(spacing: 0) {
            IconButton(label: PanesText.splitRightKey, action: { actions.split(.row) }) { _ in LucideView(icon: .columns2, size: Metric.titleGlyph) }
                .inTitleRow().answering(.newPane(.right)).disabled(!actions.canSplit(.row))
            IconButton(label: PanesText.splitDownKey, action: { actions.split(.column) }) { _ in LucideView(icon: .rows2, size: Metric.titleGlyph) }
                .inTitleRow().answering(.newPane(.below)).disabled(!actions.canSplit(.column))
        }
    }
}
