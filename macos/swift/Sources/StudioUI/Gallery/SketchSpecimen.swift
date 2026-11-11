// The first run's window sketch (`gallery/` has none of its own: it is the Look step's "How much to show" answers): the three
// presets drawn by the real `WindowSketch`, the layout the core's.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import AuxModel
import AuxWindows
import DesignSystem
import SwiftUI

struct SketchSpecimen: View {
    @Environment(\.tokens) private var tokens
    private static let presets = ["Zen", "Balanced", "Everything"]
    /// A column of the walk's card (`first_run`): wide enough for the four tabs the panel draws at most.
    private static let width = 120.0

    var body: some View {
        HStack(alignment: .top, spacing: Space._2) {
            ForEach(Array(Self.presets.enumerated()), id: \.offset) { index, name in
                VStack(alignment: .leading, spacing: Space._1) {
                    WindowSketch(blocks: SketchSource.preset(index, Self.width)).frame(width: Self.width)
                    Text(name).textRole(.caption).foregroundStyle(tokens.textMuted)
                }
            }
        }
    }
}
