// A submodule's row in Changes and under a commit (`features/git/submodule_row.rs`): the folder-with-a-repository icon, its name,
// its folder, what changed in core's words, and a button that opens a terminal in its folder (only while it is checked out
// and not deleted). It has no file menu: there is no file to open. The button shows under the pointer, inside the row's own
// button, so VoiceOver reaches it as the row's action ("Open a terminal in …"), as GPUI keeps its button in the tree.

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
                if hovering, let enter { BandButton(symbol: "terminal", label: enterLabel, inRow: true, action: enter) }
                Text(file.kind.letter).font(tokens.font(.caption).weight(.bold)).foregroundStyle(letterColor)
                    .frame(width: PanelMetrics.kindLetter, alignment: .trailing)
            }
            .padding(.leading, PanelMetrics.inset + CGFloat(depth) * Metric.indent).padding(.trailing, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
        }
        .help("\(file.path)\n\(what)")
        .accessibilityLabel(panelText("changes-spoken-submodule", ["name": .text(file.name), "what": .text(what)], "Submodule \(file.name), \(what)"))
        .accessibilityActions {
            if let enter { Button(enterLabel, action: enter) }
        }
    }

    private var enterLabel: String { panelText("submodule-enter", ["name": .text(file.name)], "Open a terminal in \(file.name)") }

    /// A terminal in the submodule's folder, while it is checked out and not deleted (nil: the row has no button).
    private var enter: (() -> Void)? {
        guard file.submoduleEnterable, let root else { return nil }
        let (host, folder) = (host, root + "/" + file.path)
        return { host?.openTerminal(in: folder, running: nil) }
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
