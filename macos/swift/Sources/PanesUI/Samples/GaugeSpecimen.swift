// What a coding agent runs and how full its context window is, as the title row writes it, for the Theme Studio's gallery
// (`studio/gallery/chrome.rs`: the agent's model and context in both meter colours): the meter in the primary, and in the
// warning colour from 80 % on, and one without a model.

import DesignSystem
import PanesModel
import SwiftUI

/// The title row's model and context meter in both colours, for the Studio's gallery.
public struct GaugeSpecimen: View {
    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        HStack(spacing: Space._4) {
            GaugeChip(gauge: GaugeFact(model: "Opus 4.5", context: 41)) // text: sample (a model's name)
            GaugeChip(gauge: GaugeFact(model: "Opus 4.5", context: 82)) // text: sample (a model's name)
            GaugeChip(gauge: GaugeFact(context: 96))
        }
    }
}
