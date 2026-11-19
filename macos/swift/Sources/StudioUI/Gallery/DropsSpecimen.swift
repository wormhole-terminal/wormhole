// The sidebar's list while a terminal is dragged (`gallery/sidebar.rs` `drops`, `navigation.rs`' drag ghost and lifted row):
// the real list caught in two drags, and the chain mark of a split whose terminals are apart in the list.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SidebarUI
import SwiftUI

struct DropsSpecimen: View {
    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            SidebarDragSpecimen()
            Noted("← a split's terminals apart in the list: the chain mark, then the notes") { ChainMarkSpecimen() }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
