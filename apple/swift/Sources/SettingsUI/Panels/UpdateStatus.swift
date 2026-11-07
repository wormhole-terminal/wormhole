// Where an update stands (`updates.rs` `draw_status`): the status in words, the button for the next step on the same row,
// a meter while downloading, and what the release brings. The updater is the app's (`UpdateViewModel`, over core's
// download, signature check and bundle swap); a build without one says what GPUI's development build says.

import AuxModel
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
                Text(status).font(.system(size: TextRole.body.size)).foregroundStyle(color).fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                actions
            }
            .frame(minHeight: SettingsMetric.rowMinHeight)
            if case .available(let release) = model.phase, !release.notes.isEmpty { notes(release) }
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
        case .available(let release): AuxText.available(release.version)
        case .unsupported(let why): why
        case .downloading: AuxText.downloading
        case .installed(let version): AuxText.ready(version)
        case .failed(let why): why
        }
    }

    private var color: Color {
        switch model.phase {
        case .failed, .unsupported: tokens.statusWarningText
        case .upToDate, .installed: tokens.statusSuccessText
        case .available(let release): release.isSecurity ? tokens.statusWarningText : tokens.textPrimary
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
            ActionButton(title: AuxText.restartNow, symbol: nil) { model.restart(whenIdle: model.restartQuestion != nil) }
        case .checking: ActionButton(title: AuxText.checkNow, symbol: Icon.refresh.rawValue, isSpinning: true) {}.disabled(true)
        case .downloading(let received, let expected): meter(received, expected)
        case .unsupported: EmptyView()
        default: ActionButton(title: AuxText.checkNow, symbol: Icon.refresh.rawValue) { model.check() }
        }
    }

    private func meter(_ received: Int64, _ expected: Int64) -> some View {
        let fraction = UpdateViewModel.fraction(received: received, expected: expected)
        return HStack(spacing: Space._2) {
            if let fraction {
                ProgressView(value: fraction).tint(tokens.primary).frame(width: SettingsMetric.meterWidth)
                Text("\(Int(fraction * 100)) %").font(.system(size: TextRole.caption.size)).monospacedDigit().foregroundStyle(tokens.textMuted)
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
            Text("\(AuxText.releaseNotes) \(release.version)").font(.system(size: TextRole.caption.size, weight: .semibold)).foregroundStyle(tokens.textMuted)
            ForEach(Array(lines(release.notes).enumerated()), id: \.offset) { _, line in
                HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                    Text("–").foregroundStyle(tokens.textFaint)
                    Text(line).foregroundStyle(tokens.textMuted)
                }
                .font(.system(size: TextRole.body.size))
            }
        }
        .padding(.vertical, Space._3)
    }

    private func lines(_ notes: String) -> [String] {
        notes.split(separator: "\n").map { $0.trimmingCharacters(in: CharacterSet(charactersIn: "-* ").union(.whitespaces)) }.filter { !$0.isEmpty }
    }
}
