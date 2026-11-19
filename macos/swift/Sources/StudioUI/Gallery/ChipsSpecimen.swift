// The title row's chips as the row draws them (branch status, context, ports, a pull request), the count badge and
// key caps beside them (`gallery/navigation.rs`); the row again with everything it can say (a longer path as the folder's
// crumb writes it, the pull request, the ports), and the agent's model and context meter in both colours.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import PanesUI
import SwiftUI

struct ChipsSpecimen: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            TitleRowView(facts: TitleSamples.agent, backend: SampleOpenIn()).frame(height: Metric.titleRow)
            TitleRowView(facts: TitleSamples.full, backend: SampleOpenIn()).frame(height: Metric.titleRow)
            Noted("← the agent's model and context, the meter in both colours") { GaugeSpecimen() }
            HStack(spacing: Space._4) {
                CountBadge(1)
                CountBadge(42)
                StatusBadge(text: "Trusted", color: tokens.statusSuccessText)
                KeyCap(text: "esc")
                KeyCap(text: "⌘K")
                KeyHintView(keys: "↩", label: "open")
            }
            // The shared chip in its states: rest, selected, outlined, disabled (`style::pill`, `neutral_chip`).
            HStack(spacing: Space._2) {
                Chip(fill: tokens.controlBackground, text: tokens.textMuted, height: Metric.titleControl) { Text("Rest").textRole(.caption) }
                Chip(fill: nil, text: tokens.textMuted, selected: true, height: Metric.titleControl) { Text("Selected").textRole(.caption) }
                Chip(fill: tokens.stateActive, text: tokens.textPrimary, outline: tokens.border, height: Metric.titleControl) { Text("Outlined").textRole(.caption) }
                Chip(fill: tokens.controlBackground, text: tokens.textMuted, height: Metric.titleControl) { Text("Disabled").textRole(.caption) }.disabled(true)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .allowsHitTesting(false)
    }
}
