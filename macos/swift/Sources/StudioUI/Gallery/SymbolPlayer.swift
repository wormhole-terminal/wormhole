// One of a symbol's motions as a button: a click plays it (a motion that plays once) or turns it to its other state (the next
// click turns it back), as the real icons do. The Animate icons setting and Reduce Motion cut it, as they cut `IconMotion`:
// a state still changes, at once, and a motion that plays once does not play. `SymbolFrames` holds the same motion still at the
// start, the middle and the end, which is the only way to see its geometry without a pointer.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

struct SymbolPlayer: View {
    let symbol: String
    let alternate: String?
    let motion: SymbolMotion
    let size: CGFloat
    let weight: Font.Weight
    let letter: String

    @State private var progress = 0.0
    @State private var run = 0
    @Environment(\.iconAnimations) private var animations
    @Environment(\.reduceMotion) private var reduceMotion

    private var plays: Bool { IconMotion.plays(animations: animations, reduceMotion: reduceMotion) }

    var body: some View {
        Button(action: play) {
            glyph
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(motion.title)
        .accessibilityLabel("\(letter): \(motion.title)")
    }

    private nonisolated func view(_ progress: Double) -> SymbolMotionView {
        SymbolMotionView(symbol: symbol, alternate: alternate, motion: motion, size: size, weight: weight, progress: progress)
    }

    @ViewBuilder private var glyph: some View {
        if motion.isState {
            view(progress)
        } else {
            // A motion that plays once rests at its end, which is how the symbol is drawn before the first click.
            Color.clear.frame(width: size, height: size).keyframeAnimator(initialValue: 1.0, trigger: run) { _, now in
                view(now)
            } keyframes: { _ in
                KeyframeTrack {
                    MoveKeyframe(0)
                    LinearKeyframe(1, duration: motion.seconds)
                }
            }
        }
    }

    private func play() {
        guard motion.isState else {
            if plays { run += 1 }
            return
        }
        let opening = progress < 0.5
        withAnimation(plays ? motion.animation(opening: opening) : nil) { progress = opening ? 1 : 0 }
    }
}

/// A motion held still at the start, the middle and the end of its progress, each in the same cell the player's symbol has.
struct SymbolFrames: View {
    let symbol: String
    let alternate: String?
    let motion: SymbolMotion
    let size: CGFloat
    let weight: Font.Weight

    var body: some View {
        HStack(spacing: 0) {
            ForEach([0.0, 0.5, 1.0], id: \.self) { progress in
                SymbolMotionView(symbol: symbol, alternate: alternate, motion: motion, size: size, weight: weight, progress: progress)
                    .frame(width: Metric.hitMin, height: Metric.hitMin)
            }
        }
    }
}
