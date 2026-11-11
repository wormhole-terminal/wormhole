// Where an update stands (`updates.rs` `draw_status`): the status in words, the button for the next step on the same row,
// a meter while downloading, and what the release brings. The updater is the app's (`UpdateViewModel`, over core's
// download, signature check and bundle swap); a build without one says what GPUI's development build says.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem
import SwiftUI
import WordsModel

extension EnvironmentValues {
    /// The app's updater, for the one Settings window; nil in a build without one.
    @Entry var settingsUpdates: UpdateViewModel?
}

struct UpdateStatusBlock: View {
    @Environment(\.settingsUpdates) private var updates
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let updates {
            UpdateStatus(model: updates)
        } else {
            PanelNote(text: Words.text("settings-updates-development-build", fallback: "Development build: updates only work in released builds."))
        }
    }
}

private struct UpdateStatus: View {
    let model: UpdateViewModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._4) {
                Text(status).textRole(.body).foregroundStyle(color).fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                actions
            }
            .frame(minHeight: SettingsMetric.rowMinHeight)
            if let note = model.channelNote { Text(note).textRole(.caption).foregroundStyle(tokens.textMuted).padding(.vertical, Space._2) }
            if case .available(let release) = model.phase {
                if let security = release.security { securityBlock(security) }
                if !release.notes.isEmpty { notes(release) }
            }
        }
        .padding(.vertical, Space._2)
        .onAppear { model.start() }
        .onDisappear { model.stop() }
        .motion(Motion.layout, value: model.phase)
        .accessibilityElement(children: .contain)
    }

    private var status: String {
        switch model.phase {
        case .idle: model.checkedLine()
        case .checking: AuxText.checking
        case .upToDate: model.lastChecked == nil ? AuxText.upToDate : "\(AuxText.upToDate) \(model.checkedLine())"
        case .available(let release): release.isSecurity ? AuxText.securityAvailable(release.version) : AuxText.available(release.version)
        case .unsupported(let why): why
        case .downloading: AuxText.downloading
        case .installed(let version): model.restartPending ? AuxText.installedWaits(version) : AuxText.ready(version)
        case .failed(let why): why
        }
    }

    private var color: Color {
        switch model.phase {
        case .failed, .unsupported: tokens.statusWarningText
        case .upToDate, .installed: tokens.statusSuccessText
        case .available(let release): release.security.map(securityColor) ?? tokens.textPrimary
        default: tokens.textMuted
        }
    }

    @ViewBuilder private var actions: some View {
        switch model.phase {
        case .available(let release):
            HStack(spacing: Space._2) {
                // A security update cannot be skipped: checks would offer it again anyway.
                if !release.isSecurity { LinkButton(title: AuxText.skipVersion) { model.skip(release) } }
                ActionButton(title: AuxText.installUpdate, symbol: nil) { model.install(release) }
            }
        case .installed:
            HStack(spacing: Space._2) {
                // Waiting for the terminals to be idle can be called off, or cut short by restarting now (no second question).
                if model.restartPending { LinkButton(title: AuxText.dontRestart) { model.cancelRestart() } }
                ActionButton(title: AuxText.restartNow, symbol: nil, action: restartNow)
            }
        case .checking: ActionButton(title: AuxText.checkNow, symbol: Icon.refresh.rawValue, isSpinning: true) {}.disabled(true)
        case .downloading(let received, let expected): meter(received, expected)
        case .unsupported: EmptyView()
        default: ActionButton(title: AuxText.checkNow, symbol: Icon.refresh.rawValue, frameId: "update.check") { model.check() }
        }
    }

    /// Restarting asks first while something runs (with "Restart When Idle" as the other way); a wait that is on goes now.
    private func restartNow() {
        guard !model.restartPending, let question = model.restartQuestion else { return model.restart(whenIdle: false) }
        Task {
            switch await QuestionSheet(question).ask(in: NSApp.keyWindow) {
            case .confirm: model.restart(whenIdle: false)
            case .also: model.restart(whenIdle: true)
            case .cancel: break
            }
        }
    }

    /// Critical and high read as danger, the rest as a warning.
    private func securityColor(_ security: Security) -> Color { security.danger ? tokens.statusDangerText : tokens.statusWarningText }

    /// What the update fixes, from the feed's (unsigned, so only advisory) severity, with each advisory as a link.
    private func securityBlock(_ security: Security) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text(security.line).textRole(.body, weight: .semibold).foregroundStyle(securityColor(security))
            if !security.advisories.isEmpty {
                HStack(spacing: Space._1) {
                    ForEach(security.advisories, id: \.id) { advisory in LinkButton(title: advisory.id) { ExternalOpen.open(advisory.url) } }
                }
            }
        }
        .padding(.vertical, Space._3)
    }

    private func meter(_ received: Int64, _ expected: Int64) -> some View {
        let fraction = UpdateViewModel.fraction(received: received, expected: expected)
        return HStack(spacing: Space._2) {
            if let fraction {
                ProgressView(value: fraction).tint(tokens.primary).frame(width: SettingsMetric.meterWidth)
                Text("\(Int(fraction * 100)) %").textRole(.caption).monospacedDigit().foregroundStyle(tokens.textMuted)
            } else {
                ProgressView().controlSize(.small)
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(AuxText.downloading)
    }

    /// What the release brings, a line each (the changelog's Markdown without its markers), under "What's new".
    private func notes(_ release: Release) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text("\(AuxText.releaseNotes) \(release.version)").textRole(.caption, weight: .semibold).foregroundStyle(tokens.textMuted)
            ForEach(Array(lines(release.notes).enumerated()), id: \.offset) { _, line in
                HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                    Text("–").foregroundStyle(tokens.textFaint)
                    Text(line).foregroundStyle(tokens.textMuted)
                }
                .textRole(.body)
            }
        }
        .padding(.vertical, Space._3)
    }

    private func lines(_ notes: String) -> [String] {
        notes.split(separator: "\n").map { $0.trimmingCharacters(in: CharacterSet(charactersIn: "-* ").union(.whitespaces)) }.filter { !$0.isEmpty }
    }
}
