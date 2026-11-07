// The Command line tool row (`sections/cli_tool.rs`): Install (or Update) and Uninstall at the right, the line saying
// where `wh` stands under them, then what needs saying (another `wh` in the way, a copy that runs another Wormhole) and
// whether new terminals will find it on their PATH, with the line to add when they will not.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct CommandLineControl: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        let records = model.host.panelRecords()
        if let tool = records.commandLine {
            VStack(alignment: .trailing, spacing: Space._2) {
                HStack(spacing: Space._2) {
                    switch tool.state {
                    case .notInstalled: install(SettingsText.install)
                    case .installed:
                        if !tool.current { install(SettingsText.update) }
                        ActionButton(title: SettingsText.uninstall, symbol: nil) { model.host.perform(.uninstallCommandLine) }
                    case .foreign: install(SettingsText.install).disabled(true)
                    }
                }
                Text(tool.headline).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                    .multilineTextAlignment(.trailing).frame(maxWidth: SettingsMetric.columnWidth / 2, alignment: .trailing)
                if let note = tool.note { notice(note, color: tool.state == .foreign ? tokens.statusWarningText : tokens.textMuted) }
                if tool.state == .installed { path(records.commandLinePath) }
            }
        } else {
            Text(SettingsText.notBuilt).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textFaint)
        }
    }

    private func install(_ title: String) -> some View {
        ActionButton(title: title, symbol: nil) { model.host.perform(.installCommandLine) }
    }

    @ViewBuilder private func path(_ note: CommandLinePathNote?) -> some View {
        if let note {
            notice(note.text, color: note.verdict == .on ? tokens.statusSuccessText : tokens.statusWarningText)
            if let line = note.line {
                CopyButton(title: SettingsText.copyLine, text: line)
            }
        } else {
            notice(SettingsText.checkingPath, color: tokens.textMuted)
        }
    }

    private func notice(_ text: String, color: Color) -> some View {
        Text(text).font(.system(size: TextRole.caption.size)).foregroundStyle(color)
            .multilineTextAlignment(.trailing).frame(maxWidth: SettingsMetric.columnWidth / 2, alignment: .trailing)
            .fixedSize(horizontal: false, vertical: true)
    }
}
