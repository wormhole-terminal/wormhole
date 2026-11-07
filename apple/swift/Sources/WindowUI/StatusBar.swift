// The thin bar under the terminal: the selected terminal's folder, branch and what it is doing. Flat, one
// hairline above it, like the GPUI app's status bar.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct StatusBar: View {
    let terminal: TerminalSummary?

    var body: some View {
        HStack(spacing: Space._3) {
            if let terminal {
                if let folder = terminal.subtitle { Label(folder, systemImage: "folder") }
                if let branch = terminal.branch { Label(branch, systemImage: "arrow.triangle.branch") }
                Spacer(minLength: 0)
                Text(terminal.activity.words)
            } else {
                Spacer(minLength: 0)
            }
        }
        .font(.caption)
        .foregroundStyle(Palette.textSecondary)
        .lineLimit(1)
        .padding(.horizontal, Space._3)
        .frame(height: Chrome.statusHeight)
        .overlay(alignment: .top) { Palette.divider.frame(height: 1) }
        .accessibilityElement(children: .combine)
    }
}

extension Activity {
    var words: String {
        switch self {
        case .idle: "Idle"
        case .working: "Working"
        case .needsInput: "Needs input"
        case .finished: "Finished"
        case .failed: "Failed"
        }
    }
}
