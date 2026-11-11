// The strip above the cards: this app, the folder, the feature's switch, the host program found, and how many hosts
// stand each way (`header.rs`).

import AuxModel
import DesignSystem
import SwiftUI

struct HostsHeader: View {
    let report: HostsReport
    let copiedAll: Bool
    let copyAll: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack {
                Text(AuxText.hostsTitle).font(TextRole.title.font).accessibilityAddTraits(.isHeader)
                Spacer()
                Button(copiedAll ? AuxText.hostsCopied : AuxText.hostsCopyAll, action: copyAll).secondaryAction().controlSize(.small)
            }
            HStack(spacing: Space._4) {
                Text(AuxText.hostsHeaderApp(version: report.appVersion, protocol: report.protocolText))
                Text("\(AuxText.hostsFolder): \(report.folder)").foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.middle)
            }
            .font(TextRole.label.font.weight(.regular))
            HStack(spacing: Space._4) {
                HStack(spacing: Space._2) {
                    Text(AuxText.hostsFlag).foregroundStyle(tokens.textMuted)
                    StatusBadge(text: report.flagOn ? AuxText.hostsOn : AuxText.hostsOff, color: report.flagOn ? tokens.statusSuccessText : tokens.statusWarningText)
                }
                HStack(spacing: Space._2) {
                    Text(AuxText.hostsBinary).foregroundStyle(tokens.textMuted)
                    Text(binary.0).foregroundStyle(binary.1).lineLimit(1).truncationMode(.middle)
                }
            }
            .font(TextRole.label.font.weight(.regular))
            HStack(spacing: Space._4) {
                tally(AuxText.hostsAttached, .attached); tally(AuxText.hostsDetached, .detached)
                tally(AuxText.hostsOrphans, .orphan); tally(AuxText.hostsStale, .stale)
            }
            if !report.flagOn {
                Text(AuxText.hostsOffNote).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            }
        }
        .padding(.horizontal, Space._4).padding(.bottom, Space._3)
    }

    private var binary: (String, Color) {
        switch report.program {
        case .found(let path): (path, tokens.textPrimary)
        case .disposableCopy: (AuxText.hostsBinaryDisposable, tokens.statusWarningText)
        case .missing: (AuxText.hostsBinaryMissing, tokens.statusWarningText)
        }
    }

    private func tally(_ label: String, _ standing: HostStanding) -> some View {
        HStack(spacing: Space._2) {
            Text(label).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            CountBadge(report.count(standing))
        }
        .accessibilityElement(children: .combine)
    }
}
