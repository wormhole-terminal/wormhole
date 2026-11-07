// The two split buttons at the row's right end: Split right (⌘D) and Split down (⇧⌘D), each greyed out
// where the focused pane has no room for a half. Folded away in a narrow row, they live in the ⋯ menu.

import DesignSystem
import PanesModel
import SwiftUI

struct SplitButtons: View {
    let actions: TitleActions

    var body: some View {
        HStack(spacing: 0) {
            TitleIconButton(symbol: "rectangle.split.2x1", label: PanesText.splitRightKey, enabled: actions.canSplit(.row)) { actions.split(.row) }
            TitleIconButton(symbol: "rectangle.split.1x2", label: PanesText.splitDownKey, enabled: actions.canSplit(.column)) { actions.split(.column) }
        }
    }
}
