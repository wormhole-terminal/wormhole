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
    /// Core's verdict on a recorded shortcut; `nil` when the model has no checker, and the plain rule stands.
    let check: (String) -> ShortcutCheck?
    let onChange: (String) -> Void
    @Environment(\.tokens) private var tokens
    @State private var recording = false
    /// Why the last shortcut was refused, in the field's words.
    @State private var refusal: String?
    private var rejected: Bool { refusal != nil }

    var body: some View {
        Button { recording.toggle() } label: {
            HStack(spacing: Space._1) {
                if recording {
                    Text(SettingsText.recorderPrompt).foregroundStyle(tokens.textMuted)
                } else if spec.isEmpty {
                    Text(SettingsText.recorderNone).foregroundStyle(tokens.textFaint)
                } else {
                    ForEach(Array(KeySpecText.caps(spec).enumerated()), id: \.offset) { _, cap in KeyCapView(text: cap) }
                }
                Spacer(minLength: 0)
            }
            .font(.system(size: TextRole.label.size))
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
        .accessibilityLabel(label)
        .accessibilityValue(spec.isEmpty ? SettingsText.recorderNone : KeySpecText.caps(spec).joined(separator: " "))
        .accessibilityHint(refusal ?? SettingsText.recorderPrompt)
        .help(refusal ?? "")
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
            .font(.system(size: TextRole.caption.size, weight: .medium))
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.keyCapMinWidth - 8, minHeight: Metric.keyCapHeight)
            .background(tokens.controlSelected, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .foregroundStyle(tokens.textPrimary)
    }
}
