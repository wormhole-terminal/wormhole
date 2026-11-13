// The profile chip beside the sidebar's toggle (and after the lights on the tab bar) and its menu (RF-21).
// Wormhole is in one profile at a time and the chip names it; it is drawn only while there are two or more.
// Its menu switches Wormhole to a profile (every window with it), opens a window in a profile or moves this
// one, and leads to the sheet that makes a profile, to Settings and to the page that manages them. A
// borderless pop-up: 24 tall, a 16 pt avatar, the name in 13, up-down chevrons, a fill only under the pointer.

import DesignSystem
import SwiftUI

public struct ProfileChoice: Identifiable, Equatable, Sendable {
    public var id: String
    public var name: String
    public var letter: Character
    /// Nil draws the primary.
    public var colour: Color?
    /// The profile Wormhole is in, and the one this window shows.
    public var current: Bool
    public var here: Bool

    public init(id: String, name: String, letter: Character, colour: Color? = nil, current: Bool, here: Bool) {
        (self.id, self.name, self.letter, self.colour, self.current, self.here) = (id, name, letter, colour, current, here)
    }
}

public struct ProfileActions {
    public var switchTo: (String) -> Void
    /// Nil leaves the entry out: a window in a profile of its own is not made yet.
    public var openWindow: ((String) -> Void)?
    public var moveWindow: ((String) -> Void)?
    public var newProfile: () -> Void
    public var manage: () -> Void
    public var settings: () -> Void

    public init(
        switchTo: @escaping (String) -> Void = { _ in }, openWindow: ((String) -> Void)? = nil,
        moveWindow: ((String) -> Void)? = nil, newProfile: @escaping () -> Void = {},
        manage: @escaping () -> Void = {}, settings: @escaping () -> Void = {}
    ) {
        (self.switchTo, self.openWindow, self.moveWindow, self.newProfile, self.manage, self.settings) =
            (switchTo, openWindow, moveWindow, newProfile, manage, settings)
    }
}

public struct ProfileChip: View {
    let choices: [ProfileChoice]
    let actions: ProfileActions
    @State private var open = false
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    public init(choices: [ProfileChoice], actions: ProfileActions = ProfileActions()) { (self.choices, self.actions) = (choices, actions) }

    public var body: some View {
        // Only drawn while there are two profiles or more to choose between.
        if choices.count > 1, let current = choices.first(where: \.current) {
            Menu { ProfileMenuItems(choices: choices, actions: actions) } label: {
                HStack(spacing: Space._2) {
                    ProfileAvatar(choice: current, size: TabMetrics.avatar)
                    Text(current.name).font(tokens.font(.label).weight(.medium)).foregroundStyle(tokens.textPrimary)
                        .lineLimit(1).frame(maxWidth: TabMetrics.chipName, alignment: .leading).fixedSize()
                    Image(systemName: "chevron.up.chevron.down").font(.system(size: Metric.chevron, weight: .semibold)).foregroundStyle(tokens.textMuted)
                }
                .padding(.leading, Space._1).padding(.trailing, Space._2)
                .frame(height: TabMetrics.chipHeight)
                .background(hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .contentShape(Rectangle())
            }
            .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden).fixedSize()
            .onHover { hovering = $0 }
            .accessibilityLabel(PanesText.profileChip(current.name))
        }
    }
}

/// A profile's avatar: its letter, bold, on its colour, in the surface's ink where that reads (4.5:1) else the
/// colour's own contrasting one; below 24 pt the letter is caption type.
struct ProfileAvatar: View {
    let choice: ProfileChoice
    let size: CGFloat
    @Environment(\.tokens) private var tokens

    /// From this size the letter is body text, below it caption text (`AVATAR_BODY_FROM`).
    private static let bodyFrom: CGFloat = 24

    var body: some View {
        Text(String(choice.letter).uppercased())
            .font(tokens.font(size >= Self.bodyFrom ? .body : .caption).weight(.bold))
            .foregroundStyle(tokens.surfaceWindow)
            .frame(width: size, height: size)
            .background(choice.colour ?? tokens.primary, in: Circle())
    }
}
