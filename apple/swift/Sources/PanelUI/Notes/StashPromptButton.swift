// The Notes tab's one action, which the window puts in the tab strip beside the panel toggle rather than on a row of its
// own above the list (`features/notes/view.rs` `stash_button`): it moves what is typed at the terminal's prompt into a
// new note, so a thought that interrupts a prompt is kept and the prompt is clear again.

import DesignSystem
import SwiftUI

public struct StashPromptButton: View {
    let controller: PanelController
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    public init(controller: PanelController) { self.controller = controller }

    public var body: some View {
        Button {
            if controller.notes.stashPrompt() { controller.tab = .notes }
        } label: {
            PanelIcon("tray.and.arrow.down", size: Metric.glyph)
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .frame(width: PanelMetrics.bandButton + 4, height: PanelMetrics.bandButton + 4)
                .background(hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain).onHover { hovering = $0 }
        .help(panelText("notes-stash-prompt", "Stash prompt into a new note"))
        .accessibilityLabel(panelText("notes-stash-prompt", "Stash prompt into a new note"))
    }
}
