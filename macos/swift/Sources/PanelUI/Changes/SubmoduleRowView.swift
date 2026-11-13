// A submodule's row in Changes and under a commit (`features/git/submodule_row.rs`): the folder-with-a-repository icon, its name,
// its folder, what changed in core's words, and a button that opens a terminal in its folder (only while it is checked out
// and not deleted). It has no file menu: there is no file to open.

import DesignSystem
import PanelModel
import SwiftUI

struct SubmoduleRowView: View {
    let file: ChangedFile
    var depth = 0
    var inTree = false
    var selected = false
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root

    var body: some View {
        let what = file.submodule ?? ""
        PanelRow(selected: selected, action: action) { hovering in
            HStack(spacing: PanelMetrics.gap) {
                PanelIcon("folder.badge.gearshape", size: Metric.glyph).foregroundStyle(muted).frame(width: Metric.lead)
                Text(file.name).font(tokens.font(.body).weight(selected ? .semibold : .regular))
                    .foregroundStyle(selected ? tokens.sidebarRowSelectedText : tokens.textPrimary).lineLimit(1)
                if !inTree && !file.folder.isEmpty {
                    Text(file.folder).font(tokens.font(.caption)).foregroundStyle(muted).lineLimit(1).truncationMode(.head)
                        .layoutPriority(-1)
                }
                Spacer(minLength: 0)
                Text(what).font(tokens.font(.caption)).foregroundStyle(muted).lineLimit(1).truncationMode(.tail).layoutPriority(-2)
                if hovering, file.submoduleEnterable, let root {
                    BandButton(symbol: "terminal", label: panelText("submodule-enter", ["name": .text(file.name)], "Open a terminal in \(file.name)"), inRow: true) {
                        host?.openTerminal(in: root + "/" + file.path, running: nil)
                    }
                }
                Text(file.kind.letter).font(tokens.font(.caption).weight(.bold)).foregroundStyle(letterColor)
                    .frame(width: PanelMetrics.kindLetter, alignment: .trailing)
            }
            .padding(.leading, PanelMetrics.inset + CGFloat(depth) * Metric.indent).padding(.trailing, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
        }
        .help("\(file.path)\n\(what)")
        .accessibilityLabel(panelText("changes-spoken-submodule", ["name": .text(file.name), "what": .text(what)], "Submodule \(file.name), \(what)"))
    }

    private var muted: Color { selected ? tokens.sidebarRowSelectedMuted : tokens.textMuted }

    private var letterColor: Color {
        switch file.kind {
        case .modified: tokens.statusWarningText
        case .added, .untracked: tokens.statusSuccessText
        case .deleted, .conflicted: tokens.statusDangerText
        case .renamed: tokens.sidebarRowSelectedText
        }
    }
}
