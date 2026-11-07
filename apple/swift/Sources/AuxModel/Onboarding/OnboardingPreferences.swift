// The few settings the Look and Agents steps change while the first run goes on (`settings` of the GPUI app:
// primary colour, terminal font and size, ligatures, notifications). Observable so the sample line redraws; the
// backend applies each change to the real settings (`onChange`), and nothing here is stored.

import Foundation
import Observation

@MainActor @Observable
public final class OnboardingPreferences {
    /// A hex colour the person picked; `nil` keeps the theme's own primary.
    public var primary: String? { didSet { onChange?() } }
    public var fontFamily: String { didSet { onChange?() } }
    public var fontSize: Double { didSet { onChange?() } }
    public var ligatures: Bool { didSet { onChange?() } }
    /// Notify when a background terminal needs the person or finishes.
    public var notifications: Bool { didSet { onChange?() } }
    /// Notify when a long command in a background terminal finishes.
    public var notifyCommandDone: Bool { didSet { onChange?() } }
    @ObservationIgnored public var onChange: (() -> Void)?

    public init(
        primary: String? = nil, fontFamily: String = "Menlo", fontSize: Double = 13, ligatures: Bool = true,
        notifications: Bool = true, notifyCommandDone: Bool = true
    ) {
        (self.primary, self.fontFamily, self.fontSize, self.ligatures) = (primary, fontFamily, fontSize, ligatures)
        (self.notifications, self.notifyCommandDone) = (notifications, notifyCommandDone)
    }
}
