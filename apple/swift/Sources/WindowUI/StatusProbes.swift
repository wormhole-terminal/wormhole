// What a Probe's hook set for the status bar (`window/status_bar.rs`, "A Probe's items"): its text in the tone it chose (a
// design token, never a colour), its name in the tooltip and in what VoiceOver says. Rust sets and caps the text (80
// characters, made visible); this only draws it.

import DesignSystem
import SwiftUI

public struct ProbeStatusItem: Identifiable, Equatable, Sendable {
    /// `<probe>/<item>`.
    public var id: String { key }
    public let key: String
    public let text: String
    /// The design token it is drawn with (`status.danger.text`); nil keeps the bar's own text colour.
    public let token: String?
    /// The Probe's name, then whatever tooltip it gave.
    public let tooltip: String

    public init(key: String, text: String, token: String?, tooltip: String) {
        (self.key, self.text, self.token, self.tooltip) = (key, text, token, tooltip)
    }
}

struct StatusProbeItems: View {
    let items: [ProbeStatusItem]
    @Environment(\.tokens) private var tokens
    @Environment(\.chrome) private var colours

    var body: some View {
        HStack(spacing: Space._2) {
            ForEach(items) { item in
                Text(item.text)
                    .lineLimit(1)
                    .foregroundStyle(item.token.map { tokens.color(named: $0) } ?? colours.muted)
                    .help(item.tooltip)
                    .accessibilityLabel("\(item.tooltip): \(item.text)")
            }
        }
    }
}
