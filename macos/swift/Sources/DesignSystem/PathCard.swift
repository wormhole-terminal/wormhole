// A path's hover card: a popover under a path in the title row (a terminal's folder, the file a viewer shows) with the
// whole path and its buttons: Open in Finder, Copy path (which says "Copied" for a moment) and Insert into terminal.
// The owner says what the buttons are and do; the card draws them. Esc closes it (the popover does).

import SwiftUI

/// One button of a path card.
public struct PathCardButton: Identifiable {
    public let id: String
    public var symbol: String
    public var title: String
    public var enabled: Bool
    /// What a copying button says for a moment after it was pressed ("Copied"); such a button leaves the card up.
    public var copied: String?
    public var action: @MainActor () -> Void

    public init(id: String, symbol: String, title: String, enabled: Bool = true, copied: String? = nil, action: @escaping @MainActor () -> Void) {
        (self.id, self.symbol, self.title, self.enabled, self.copied, self.action) = (id, symbol, title, enabled, copied, action)
    }
}

public struct PathCard: View {
    let path: String
    let buttons: [PathCardButton]
    let hover: PathCardHover
    @State private var copied = false
    @Environment(\.tokens) private var tokens

    /// How long "Copied" stays (`PATH_CARD_COPIED_FOR`).
    private static let copiedFor: Duration = .milliseconds(1200)

    public init(path: String, buttons: [PathCardButton], hover: PathCardHover) {
        (self.path, self.buttons, self.hover) = (path, buttons, hover)
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Text(path).font(tokens.font(.body)).foregroundStyle(tokens.textPrimary).textSelection(.enabled)
                .lineLimit(3).fixedSize(horizontal: false, vertical: true)
            HStack(spacing: Space._1) {
                ForEach(buttons) { button in
                    let said = copied ? button.copied : nil
                    cardButton(said == nil ? button.symbol : "checkmark", said ?? button.title, enabled: button.enabled) {
                        if button.copied != nil { copy(button) } else { hover.close(); button.action() }
                    }
                }
            }
        }
        .padding(Space._3)
        .frame(minWidth: 240, maxWidth: 360, alignment: .leading)
        .onHover { hover.hover(.card, $0) }
    }

    private func cardButton(_ symbol: String, _ label: String, enabled: Bool = true, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label(label, systemImage: symbol).font(tokens.font(.caption)).labelStyle(.titleAndIcon)
        }
        .buttonStyle(.bordered).controlSize(.small).disabled(!enabled)
    }

    private func copy(_ button: PathCardButton) {
        button.action()
        copied = true
        Task {
            // one-shot: "Copied" goes back to the button's own words after its moment.
            try? await Task.sleep(for: Self.copiedFor)
            copied = false
        }
    }
}
