// The Extensions page (`sections/extensions.rs`): the Probes in the Probes folder, one row each with its state and a
// switch. Switching one on opens the approval sheet; off needs no question. The folder is read when the page opens and
// from the refresh button and when the window comes forward, never on a timer.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI

struct ProbesPanel: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let probes = model.probesModel { ProbesList(probes: probes) } else { PanelNote(text: SettingsText.notBuilt) }
    }
}

private struct ProbesList: View {
    @Bindable var probes: ProbesModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            if let notice = probes.notice { Banner(kind: .danger, text: notice).padding(.vertical, Space._2) }
            if let list = probes.list {
                if let tooMany = list.tooMany {
                    Banner(kind: .warning, text: tooMany).padding(.vertical, Space._2)
                } else {
                    if let problem = list.keyProblem { Banner(kind: .warning, text: problem).padding(.vertical, Space._2) }
                    if list.rows.isEmpty { empty(list) }
                    ForEach(Array(list.rows.enumerated()), id: \.element.id) { index, row in
                        if index > 0 { RowDivider() }
                        ProbeRowView(row: row, probes: probes)
                    }
                }
            } else {
                PanelNote(text: ExtensionsText.looking)
            }
        }
        .task { await probes.refresh() }
        // The folder is read again when the window comes forward with the page showing (a Probe was added or changed outside).
        .onReceive(NotificationCenter.default.publisher(for: NSWindow.didBecomeKeyNotification)) { note in
            if (note.object as? NSWindow)?.title == SettingsText.settings { Task { await probes.refresh() } }
        }
        .sheet(
            isPresented: Binding(get: { probes.open != nil }, set: { if !$0 { probes.close() } })
        ) {
            ProbeSheetView(probes: probes)
        }
    }

    private func empty(_ list: ProbeList) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Image(Icon.extensions).font(.system(size: Metric.glyph)).foregroundStyle(tokens.textMuted)
            Text(ExtensionsText.noneTitle).textRole(.body, weight: .semibold).foregroundStyle(tokens.textPrimary)
            Text(ExtensionsText.noneWhere(list.folder) + " " + ExtensionsText.noneNew("wh probe new")) // text: shell command
                .textRole(.caption).foregroundStyle(tokens.textMuted)
            ActionButton(title: ExtensionsText.showFolder, symbol: Icon.folder.rawValue) { Self.reveal(list.folderPath) }.padding(.top, Space._1)
        }
        .padding(.vertical, Space._3)
    }

    /// The Probes folder in the Finder; made first, since nothing has put a Probe there yet.
    private static func reveal(_ path: String) {
        let url = URL(fileURLWithPath: path)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        ExternalOpen.open(url)
    }
}
