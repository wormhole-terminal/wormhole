// The terminal's status tile in every state at the three sizes, and the tab bar the tiles' terminals sit in
// (`gallery/tiles.rs`): the same views the panes' headers and the title row draw.

import DesignSystem
import PanesUI
import SwiftUI
import WorkspaceModel

struct TilesSpecimen: View {
    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            ForEach([Metric.tile, Metric.tileMini, Metric.tileSmall], id: \.self) { size in
                HStack(spacing: Space._4) {
                    ForEach([Activity.idle, .working, .needsInput, .finished, .failed, .exited], id: \.self) {
                        TerminalTile(activity: $0, size: size)
                    }
                }
            }
            TabBarView(model: TabSamples.bar(), profiles: TabSamples.profiles)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .allowsHitTesting(false)
    }
}
