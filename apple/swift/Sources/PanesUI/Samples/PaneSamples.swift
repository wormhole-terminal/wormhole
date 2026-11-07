// Sample data for previews and snapshot tests: three terminals in a split, and a stand-in for a terminal's
// own view (a block of prompt text on the terminal's background).

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

public enum PaneSamples {
    public static let info: [PaneID: PaneInfo] = [
        1: PaneInfo(title: "zsh", folder: "~/Projects/tube"),
        2: PaneInfo(title: "claude", folder: "~/Projects/tube/crates/core", activity: .working, agent: .claude),
        3: PaneInfo(title: "cargo watch", folder: "~/Projects/tube", activity: .needsInput),
        4: PaneInfo(title: "build docs", folder: "~", activity: .finished),
    ]

    /// 1 on the left, 2 above 3 on the right.
    public static var split: TabPanes {
        var panes = TabPanes.single(1)
        _ = panes.split(.row, new: 2)
        _ = panes.split(.column, new: 3)
        _ = panes.focus(2)
        return panes
    }

    public static var grid: TabPanes {
        var panes = TabPanes.single(1)
        _ = panes.arrange(.grid, newIDs: [2, 3, 4])
        return panes
    }
}

/// A terminal's place in a preview: its background and a few lines of text in the terminal's colours.
public struct SampleTerminal: View {
    let id: PaneID
    @Environment(\.tokens) private var tokens

    public init(id: PaneID) { self.id = id }

    public var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("~/Projects/tube % cargo test").foregroundStyle(tokens.terminal.foreground.color)
            Text("   Compiling tube v0.4.1 (pane \(id))").foregroundStyle(tokens.terminal.foreground.color.opacity(0.6))
            Text("~/Projects/tube % ▍").foregroundStyle(tokens.terminal.foreground.color)
            Spacer(minLength: 0)
        }
        .font(.system(size: 12, design: .monospaced))
        .padding(.horizontal, 4).padding(.top, 2)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(tokens.terminal.background.color, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg - PaneMetrics.inset / 2, style: .continuous))
    }
}
