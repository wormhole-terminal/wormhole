// The terminal's status tile in every state at the three sizes, and the tab bar the tiles' terminals sit in
// (`gallery/tiles.rs`): the same views the panes' headers and the title row draw. Under them the list's own tile (the agent's
// logo, the light, the badge) in every state with a status change to play, and a count turning over: the Swift app has no
// rolling number of its own, a count badge's digits turn by the system's numeric transition, which cannot be held half way.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import PanesUI
import SidebarUI
import SwiftUI
import WorkspaceModel

struct TilesSpecimen: View {
    @State private var count = 12

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            ForEach([Metric.tile, Metric.tileMini, Metric.tileSmall], id: \.self) { size in
                HStack(spacing: Space._4) {
                    ForEach([Activity.idle, .working, .needsInput, .finished, .failed, .exited], id: \.self) {
                        TerminalTile(activity: $0, size: size)
                    }
                }
            }
            .allowsHitTesting(false)
            TileStatesSpecimen()
            HStack(spacing: Space._3) {
                CountBadge(count)
                CountBadge(count + 7)
                Button(StudioWords.turnCount) { count += 1 }
            }
            TabBarView(model: TabSamples.bar(), profiles: TabSamples.profiles).allowsHitTesting(false)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
