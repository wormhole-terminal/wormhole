// An icon answering with its own drawing (`style::icon_motion`): a + that turns a quarter, arrows that spin, a copy that
// fills and becomes a check, a split icon filling the half its new pane takes, one icon shrinking away for the next, a
// chevron turned over with its menu. The icon never moves or shakes as a whole, and each motion ends on the icon's usual
// look. The Animate icons setting (`\.iconAnimations`) and Reduce Motion (`\.reduceMotion`) switch them all off: a start
// is only counted while they may play, so what was missed is not played when they come back on.

import SwiftUI

public enum IconMotion: Equatable, Sendable {
    /// On a click, a quarter turn: a + looks the same after it.
    case quarter
    /// On a click, a whole turn the way the arrows curl.
    case spin(clockwise: Bool)
    /// On a click, the copy icon fills and turns into a check, which turns back.
    case copied
    /// On a click, the half of a split icon the new pane takes fills and lets go.
    case newPane(Half)

    public enum Half: Equatable, Sendable { case right, below }

    /// The motion a symbol has wherever it is a button, so the same icon answers the same way everywhere.
    public static func of(symbol: String) -> IconMotion? {
        switch symbol {
        case Icon.add.rawValue: .quarter
        case Icon.refresh.rawValue: .spin(clockwise: true)
        case Icon.reset.rawValue: .spin(clockwise: false)
        case Icon.copy.rawValue: .copied
        default: nil
        }
    }

    /// The same, for a Lucide drawing.
    public static func of(_ icon: Lucide) -> IconMotion? {
        switch icon {
        case .plus: .quarter
        case .refreshCw: .spin(clockwise: true)
        case .copy: .copied
        default: nil
        }
    }

    /// Whether a motion may play: Animate icons is on and Reduce Motion is not.
    public static func plays(animations: Bool, reduceMotion: Bool) -> Bool { animations && !reduceMotion }

    /// How long each takes, in seconds (`icon_motion/*`).
    enum Seconds {
        static let plusTurn = 0.22, arrowsSpin = 0.52, copied = 1.4, paneFill = 0.48, swap = 0.24, chevronTurn = 0.2
    }
}

/// How many times a button's motion has started, 0 while it has not: a button keeps one and starts it from its action.
public struct IconRun: DynamicProperty {
    @Environment(\.iconAnimations) private var animations
    @Environment(\.reduceMotion) private var reduceMotion
    @State private var starts = 0

    public init() {}

    public var count: Int { starts }

    /// A click: counted only where motion plays.
    public func start() {
        if IconMotion.plays(animations: animations, reduceMotion: reduceMotion) { starts += 1 }
    }
}

extension View {
    /// This glyph answering the `run`th start of `motion` (0 leaves it at rest); no motion leaves it alone.
    public func iconMotion(_ motion: IconMotion?, run: Int) -> some View { modifier(IconMotionModifier(motion: motion, run: run)) }

    /// A menu's chevron, turned over while the menu is `open` (down at rest). Animate icons off or Reduce Motion cuts it.
    public func menuChevron(open: Bool) -> some View { modifier(MenuChevronModifier(open: open)) }
}

private struct IconMotionModifier: ViewModifier {
    let motion: IconMotion?
    let run: Int

    func body(content: Content) -> some View {
        switch motion {
        case nil: content
        case .quarter: turning(content, degrees: 90, seconds: IconMotion.Seconds.plusTurn, easeOut: true)
        case .spin(let clockwise): turning(content, degrees: clockwise ? 360 : -360, seconds: IconMotion.Seconds.arrowsSpin, easeOut: false)
        case .copied: copied(content)
        case .newPane(let half): filling(content, half)
        }
    }

    /// Turned by `degrees` by the end, which looks like rest for a + and a whole turn. The + decelerates, as it only turns a quarter.
    private func turning(_ content: Content, degrees: Double, seconds: Double, easeOut: Bool) -> some View {
        content.keyframeAnimator(initialValue: 0.0, trigger: run) { glyph, angle in
            glyph.rotationEffect(.degrees(angle))
        } keyframes: { _ in
            KeyframeTrack { CubicKeyframe(degrees, duration: seconds, startVelocity: easeOut ? 2.5 * degrees / seconds : 0) }
        }
    }

    private struct Copy {
        var sheet = 0.0, copy = 1.0, check = 0.0, scale = 1.0
    }

    /// One linear timeline: the sheet fills (to 12 %), lets go as the check takes over (22 %), which holds until 86 % and gives
    /// the copy icon back.
    private func copied(_ content: Content) -> some View {
        let total = IconMotion.Seconds.copied
        func at(_ share: Double, _ gap: Double) -> Double { (share - gap) * total }
        return content.keyframeAnimator(initialValue: Copy(), trigger: run) { glyph, now in
            glyph.opacity(now.copy)
                .background { LucidePart.Piece.copySheet.opacity(now.sheet) }
                .overlay { LucidePart.Piece.check.scaleEffect(now.scale).opacity(now.check) }
        } keyframes: { _ in
            KeyframeTrack(\.sheet) {
                LinearKeyframe(1, duration: at(0.12, 0)); LinearKeyframe(0, duration: at(0.22, 0.12))
            }
            KeyframeTrack(\.copy) {
                LinearKeyframe(1, duration: at(0.12, 0)); LinearKeyframe(0, duration: at(0.22, 0.12))
                LinearKeyframe(0, duration: at(0.86, 0.22)); LinearKeyframe(1, duration: at(1, 0.86))
            }
            KeyframeTrack(\.check) {
                LinearKeyframe(0, duration: at(0.12, 0)); LinearKeyframe(1, duration: at(0.22, 0.12))
                LinearKeyframe(1, duration: at(0.86, 0.22)); LinearKeyframe(0, duration: at(1, 0.86))
            }
            KeyframeTrack(\.scale) {
                MoveKeyframe(0.7); LinearKeyframe(0.7, duration: at(0.12, 0)); LinearKeyframe(1, duration: at(0.22, 0.12))
            }
        }
    }

    private struct Fill {
        var grown = 0.0, strength = 0.0
    }

    /// The half of the frame the new pane takes grows from the divider outward (decelerating, to 45 %), then lets go.
    private func filling(_ content: Content, _ half: IconMotion.Half) -> some View {
        let total = IconMotion.Seconds.paneFill
        return content.keyframeAnimator(initialValue: Fill(), trigger: run) { glyph, now in
            let edge = 12 + 9 * now.grown
            let cut = half == .right ? LucidePart.cut(12, 3, edge, 21) : LucidePart.cut(3, 12, 21, edge)
            glyph.background { LucidePart.Piece.frameFill(cut: cut).opacity(now.strength) }
        } keyframes: { _ in
            KeyframeTrack(\.grown) {
                MoveKeyframe(0); CubicKeyframe(1, duration: 0.45 * total, startVelocity: 2.5 / (0.45 * total))
            }
            KeyframeTrack(\.strength) {
                MoveKeyframe(1); LinearKeyframe(1, duration: 0.45 * total); CubicKeyframe(0, duration: 0.55 * total)
            }
        }
    }
}

private struct MenuChevronModifier: ViewModifier {
    let open: Bool
    @Environment(\.iconAnimations) private var animations
    @Environment(\.reduceMotion) private var reduceMotion

    /// A menu opens as things arrive (decelerating) and closes as they leave.
    func body(content: Content) -> some View {
        let turn = IconMotion.plays(animations: animations, reduceMotion: reduceMotion)
            ? (open ? Animation.easeOut : Animation.easeIn)(IconMotion.Seconds.chevronTurn) : nil
        content.rotationEffect(.degrees(open ? 180 : 0)).animation(turn, value: open)
    }
}

/// One icon swapped for another (a pane's zoom button, its corners turning from out to in): the old one shrinks away and
/// the new one grows into its place. The change of `key` drives it, not a click, so a zoom made by the keys shows it too.
/// Animate icons off or Reduce Motion cuts it.
public struct IconSwap<Key: Hashable, Glyph: View>: View {
    let key: Key
    let glyph: (Key) -> Glyph
    @Environment(\.iconAnimations) private var animations
    @Environment(\.reduceMotion) private var reduceMotion

    public init(key: Key, @ViewBuilder glyph: @escaping (Key) -> Glyph) { (self.key, self.glyph) = (key, glyph) }

    /// How small the icons get at the turn.
    private static var smallest: Double { 0.6 }

    public var body: some View {
        let half = IconMotion.Seconds.swap / 2
        let small = AnyTransition.scale(scale: Self.smallest).combined(with: .opacity)
        ZStack {
            glyph(key).id(key).transition(.asymmetric(
                insertion: small.animation(.easeOut(duration: half).delay(half)), removal: small.animation(.easeOut(duration: half))))
        }
        .animation(IconMotion.plays(animations: animations, reduceMotion: reduceMotion) ? .default : nil, value: key)
    }
}
