// A row's button that does something (open a folder, put things back): the system's bordered button with its symbol, and
// the link-style button that takes you to another page (`style::button`, `style::link_button`).

import DesignSystem
import SwiftUI

struct ActionButton: View {
    let title: String
    let symbol: String?
    var isDestructive = false
    /// The symbol turns while something this button started is running (Reduce Motion keeps it still).
    var isSpinning = false
    let action: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Button(role: isDestructive ? .destructive : nil, action: action) {
            if let symbol { Label(title, systemImage: symbol).symbolEffect(.rotate, isActive: isSpinning && !reduceMotion) } else { Text(title) }
        }
        .buttonStyle(.bordered)
    }
}

struct LinkButton: View {
    let title: String
    var goes = false
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._1) {
                Text(title)
                if goes { Image(Icon.goTo).imageScale(.small) }
            }
        }
        .buttonStyle(.link)
        .foregroundStyle(tokens.textLink)
    }
}
