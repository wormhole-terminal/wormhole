// The shortcut recorder row (`recorder.rs`): the keys as caps; a click starts recording (the field says what to
// press), the next shortcut is the value, Esc cancels and ⌫ clears. The key capture is an `NSView` because
// SwiftUI hands Esc, Tab and ⌘-keys to the focus system before a view can see them.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI

struct ShortcutRecorder: View {
    let spec: String
    let label: String
    /// Why the shortcut set now does not work, shown in the field while it is not recording (a conflict set by hand in the file, or
    /// a key the system did not register).
    var problem: String?
    /// Core's verdict on a recorded shortcut; `nil` when the model has no checker, and the plain rule stands.
    let check: (String) -> ShortcutCheck?
    let onRecording: (Bool) -> Void
    let onChange: (String) -> Void
    @Environment(\.tokens) private var tokens
    @State private var recording = false
    /// Why the last shortcut was refused, in the field's words.
    @State private var refusal: String?
    private var shownProblem: String? { refusal ?? (recording ? nil : problem) }
    private var rejected: Bool { shownProblem != nil }

    var body: some View {
        Button { recording.toggle() } label: {
            HStack(spacing: Space._1) {
                if let refusal = shownProblem {
                    // Keys that macOS, the app or the shell already answer to are said, not taken (`Recording::Taken`).
                    Text(refusal).lineLimit(1).foregroundStyle(tokens.statusWarningText)
                } else if recording {
                    Text(SettingsText.recorderPrompt).foregroundStyle(tokens.textMuted)
                } else if spec.isEmpty {
                    Text(SettingsText.recorderNone).foregroundStyle(tokens.textFaint)
                } else {
                    ForEach(Array(KeySpecText.caps(spec).enumerated()), id: \.offset) { _, cap in KeyCapView(text: cap) }
                }
                Spacer(minLength: 0)
            }
            .textRole(.label)
            .padding(.horizontal, Space._2)
            .frame(width: SettingsMetric.controlWidth, height: Metric.control)
            .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay {
                RoundedRectangle(cornerRadius: tokens.metrics.radius)
                    .strokeBorder(rejected ? tokens.statusDanger : tokens.inputBorderFocus, lineWidth: recording || rejected ? tokens.metrics.focusRingWidth : 0)
            }
            .background { if recording { KeyCapture(onKey: record) } }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .overlay(alignment: .trailing) {
            // Delete clears, and so does the ×.
            if !spec.isEmpty, !recording {
                IconButton(symbol: Icon.remove.rawValue, label: SettingsText.clearSearch) { onChange("") }
                    .frame(width: Metric.controlSmall, height: Metric.controlSmall).padding(.trailing, Space._1)
            }
        }
        .marked("recorder")
        .onChange(of: recording) { onRecording(recording) }
        .onDisappear { if recording { onRecording(false) } }
        .accessibilityLabel(label)
        .accessibilityValue(spec.isEmpty ? SettingsText.recorderNone : KeySpecText.caps(spec).joined(separator: " "))
        .accessibilityHint(shownProblem ?? SettingsText.recorderPrompt)
        .help(shownProblem ?? "")
    }

    private func record(_ key: RecordedKey) {
        switch key {
        case .cancel: finish()
        case .clear:
            onChange("")
            finish()
        case .spec(let spec):
            switch check(spec) ?? (KeySpecText.isUsable(spec) ? .accepted(spec) : .needsModifier) {
            case .accepted(let stored):
                onChange(stored)
                finish()
            case .cancel: finish()
            case .clear:
                onChange("")
                finish()
            case .needsModifier: refusal = SettingsText.recorderPrompt
            case .refused(let reason): refusal = reason
            }
        }
    }

    private func finish() {
        refusal = nil
        recording = false
    }
}

private struct KeyCapView: View {
    let text: String
    @Environment(\.tokens) private var tokens
    var body: some View {
        Text(text)
            .textRole(.caption, weight: .medium)
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.keyCapMinWidth - 8, minHeight: Metric.keyCapHeight)
            .background(tokens.controlSelected, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .foregroundStyle(tokens.textPrimary)
    }
}
