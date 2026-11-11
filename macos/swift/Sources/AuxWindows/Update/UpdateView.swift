// The Updates window: the version running, what a check found, the channel (Stable or Preview), the release's
// notes, and the buttons that carry an update through: Check now, Install Update, Skip This Version, then Restart
// now or when idle. Core downloads, verifies the signature and swaps the bundle; this is the face of it
// (`UpdateBackend`). Where the restart would end something, a system sheet lists it first.

import AppKit
import AuxModel
import DesignSystem
import SwiftUI

struct UpdateView: View {
    let model: UpdateViewModel
    let version: String
    /// The window the restart question sheets onto.
    let sheetWindow: () -> NSWindow?
    @Environment(\.tokens) private var tokens
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            header
            status
            if case .available(let release) = model.phase { notes(release) }
            Spacer(minLength: 0)
            Rule()
            channelRow
        }
        .padding(Space._6)
        .auxPage()
        .onAppear { model.start() }
        .onDisappear { model.stop() }
        .motion(Motion.layout, value: model.phase)
    }

    private var header: some View {
        HStack(spacing: Space._3) {
            Image(nsImage: NSApp.applicationIconImage).resizable().frame(width: Metric.tile * 2, height: Metric.tile * 2).accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 0) {
                Text(AuxText.brand).font(TextRole.heading.font).accessibilityAddTraits(.isHeader)
                Text("\(AuxText.updatesVersion) \(version)").font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            }
            Spacer()
        }
    }

    @ViewBuilder private var status: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            switch model.phase {
            case .idle, .upToDate:
                line(model.phase == .upToDate ? AuxText.upToDate : model.checkedLine(), symbol: model.phase == .upToDate ? "checkmark.circle" : nil)
                Button(AuxText.checkNow) { model.check() }.secondaryAction()
            case .checking:
                HStack(spacing: Space._2) { ProgressView().controlSize(.small); Text(AuxText.checking).foregroundStyle(tokens.textMuted) }
            case .available(let release):
                line(AuxText.available(release.version), symbol: "arrow.down.circle")
                if release.isSecurity { StatusBadge(text: AuxText.securityUpdate, color: tokens.statusWarningText) }
                HStack {
                    Button(AuxText.installUpdate) { model.install(release) }.primaryAction().keyboardShortcut(.defaultAction)
                    Button(AuxText.skipVersion) { model.skip(release) }.buttonStyle(.plain).foregroundStyle(tokens.textMuted)
                }
            case .downloading(let received, let expected):
                downloading(received, expected)
            case .installed(let version):
                line(AuxText.ready(version), symbol: "checkmark.circle")
                HStack {
                    Button(AuxText.restartNow, action: restart).primaryAction().keyboardShortcut(.defaultAction)
                    Button(AuxText.restartWhenIdleButton) { model.restart(whenIdle: true) }.secondaryAction()
                }
            case .unsupported(let why): line(why, symbol: "info.circle")
            case .failed(let why):
                line(AuxText.failed(why), symbol: "exclamationmark.triangle", color: tokens.statusDangerText)
                Button(AuxText.checkNow) { model.check() }.secondaryAction()
            }
        }
        .font(TextRole.body.font)
    }

    private func line(_ text: String, symbol: String?, color: Color? = nil) -> some View {
        Label { Text(text) } icon: { if let symbol { Image(systemName: symbol) } }
            .labelStyle(.titleAndIcon).foregroundStyle(color ?? tokens.textPrimary)
            .accessibilityElement(children: .combine)
    }

    private func downloading(_ received: Int64, _ expected: Int64) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text(AuxText.downloading).foregroundStyle(tokens.textMuted)
            if let fraction = UpdateViewModel.fraction(received: received, expected: expected) {
                ProgressView(value: fraction).tint(tokens.primary).progressViewStyle(.linear)
                if let text = UpdateViewModel.progressText(received: received, expected: expected) {
                    Text(text).font(TextRole.caption.font.monospacedDigit()).foregroundStyle(tokens.textMuted)
                }
            } else {
                ProgressView().progressViewStyle(.linear)
            }
        }
        .accessibilityElement(children: .combine)
    }

    private func notes(_ release: Release) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            SectionLabel(AuxText.releaseNotes)
            ScrollView {
                if release.notes.isEmpty {
                    Text(AuxText.noNotes).foregroundStyle(tokens.textMuted).frame(maxWidth: .infinity, alignment: .leading)
                } else {
                    ReleaseNotesView(notes: release.notes)
                }
            }
            .frame(maxHeight: .infinity)
            .padding(Space._3)
            .background(tokens.surfacePanel, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
        }
    }

    private var channelRow: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            HStack {
                Text(AuxText.updatesChannel).font(TextRole.body.font)
                Spacer()
                Picker(AuxText.updatesChannel, selection: Bindable(model).channel) {
                    Text(AuxText.channelStable).tag(UpdateChannel.stable)
                    Text(AuxText.channelPreview).tag(UpdateChannel.preview)
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize()
            }
            Text(AuxText.channelNote).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
        }
    }

    /// Restart now: at once when nothing would end, else the question first, with "Restart When Idle".
    private func restart() {
        guard let question = model.restartQuestion else { return model.restart(whenIdle: false) }
        Task {
            switch await QuestionSheet(question).ask(in: sheetWindow()) {
            case .confirm: model.restart(whenIdle: false)
            case .also: model.restart(whenIdle: true)
            case .cancel: break
            }
        }
    }
}
