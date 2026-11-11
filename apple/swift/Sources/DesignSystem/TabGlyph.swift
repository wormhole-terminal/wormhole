// A right panel tab's glyph playing once as its tab becomes the active one (`icon_motion/tab_glyph`): each icon does what
// it is about, drawn from pieces of the icon itself laid over it. Git-compare's two commits fill in turn, the commit's dot
// fills as its lines draw out, the gauge's needle sweeps up, the page's lines are written, the folder flashes full. Each
// ends on the plain icon, which is also how it is drawn before its first run: the picture is the icon's pieces, whose rest
// frame is the icon. Which one an icon plays is the icon's business, so a tab bar needn't know what a tab is.

import SwiftUI

/// The tab icons that play something: compare, commit, gauge, file-text and folder. Another icon is drawn plainly.
public struct TabGlyph: View {
    let icon: Lucide
    let size: CGFloat
    /// How many times the tab has become active (an `IconRun`'s count): 0 is at rest.
    let run: Int

    public init(icon: Lucide, size: CGFloat, run: Int) { (self.icon, self.size, self.run) = (icon, size, run) }

    /// What each piece of the picture is doing, 0 to 1 (the gauge's first is an angle): which is which depends on the icon.
    private struct Beat {
        var first = 0.0, second = 0.0, third = 0.0
    }

    /// An ease-out keyframe: decelerating, as `EASE_OUT`.
    private func lift(_ to: Double, _ seconds: Double) -> CubicKeyframe<Double> {
        CubicKeyframe(to, duration: seconds, startVelocity: 2.5 / seconds)
    }

    public var body: some View {
        switch icon {
        case .gitCompare: compare
        case .commit: commit
        case .gauge: gauge
        case .fileText: notes
        case .folder: folder
        default: LucideView(icon: icon, size: size)
        }
    }

    /// Both circles fill in turn, the second following the first; the circles at (6, 6) and (18, 18) are the one dot shifted.
    private var compare: some View {
        let total = 0.6
        return LucideView(icon: .gitCompare, size: size).keyframeAnimator(initialValue: Beat(), trigger: run) { glyph, now in
            glyph
                .background { LucidePart.Piece.dot(at: CGSize(width: -6, height: -6)).opacity(now.first) }
                .background { LucidePart.Piece.dot(at: CGSize(width: 6, height: 6)).opacity(now.second) }
        } keyframes: { _ in
            KeyframeTrack(\.first) {
                LinearKeyframe(1, duration: 0.35 * total); LinearKeyframe(1, duration: 0.35 * total); LinearKeyframe(0, duration: 0.3 * total)
            }
            KeyframeTrack(\.second) {
                LinearKeyframe(0, duration: 0.25 * total); LinearKeyframe(1, duration: 0.35 * total)
                LinearKeyframe(1, duration: 0.1 * total); LinearKeyframe(0, duration: 0.3 * total)
            }
        }
    }

    /// The dot fills while the lines draw out from the ring (480 ms). The lines run x 3 to 9 and 15 to 21; one not yet drawn is
    /// hidden by the ring's stroke, which reaches x 8 to 16.
    private var commit: some View {
        let total = 0.48
        let (left, right) = (Lucide.panelDrawings[.commit]?[1] ?? "", Lucide.panelDrawings[.commit]?[2] ?? "")
        return Color.clear.frame(width: size, height: size).keyframeAnimator(initialValue: Beat(second: 1), trigger: run) { _, now in
            ZStack {
                LucidePart.Piece.dot().opacity(now.first)
                LucidePart.Piece.ring
                LucidePart.Piece.line(left, cut: LucidePart.cut(10 - 9 * now.second, 0, 10, 24))
                LucidePart.Piece.line(right, cut: LucidePart.cut(14, 0, 14 + 9 * now.second, 24))
            }
            .frame(width: size, height: size)
        } keyframes: { _ in
            KeyframeTrack(\.first) {
                LinearKeyframe(1, duration: 0.3 * total); LinearKeyframe(1, duration: 0.35 * total); LinearKeyframe(0, duration: 0.35 * total)
            }
            KeyframeTrack(\.second) { MoveKeyframe(0); lift(1, 0.6 * total) }
        }
    }

    /// The needle starts turned back 135 degrees, swings 6 % past its place at 70 % and settles (520 ms).
    private var gauge: some View {
        let total = 0.52
        return Color.clear.frame(width: size, height: size).keyframeAnimator(initialValue: Beat(), trigger: run) { _, now in
            ZStack {
                LucidePart.Piece.gaugeArc
                LucidePart.Piece.gaugeNeedle(degrees: now.first)
            }
            .frame(width: size, height: size)
        } keyframes: { _ in
            KeyframeTrack(\.first) {
                MoveKeyframe(-135)
                CubicKeyframe(0.06 * 135, duration: 0.7 * total, startVelocity: 2.5 * 143.1 / (0.7 * total))
                CubicKeyframe(0, duration: 0.3 * total)
            }
        }
    }

    /// The page's three lines are written one after the other, each a fifth of the run after the one above (480 ms). The
    /// lines start at x 8, their round ends reaching a unit past on each side; the first is short.
    private var notes: some View {
        let total = 0.48
        let lines = [("M10 9H8", 10.0), ("M16 13H8", 16.0), ("M16 17H8", 16.0)]
        return Color.clear.frame(width: size, height: size).keyframeAnimator(
            initialValue: Beat(first: 1, second: 1, third: 1), trigger: run
        ) { _, now in
            let written = [now.first, now.second, now.third]
            ZStack {
                LucidePart.Piece.page
                ForEach(lines.indices, id: \.self) { line in
                    LucidePart.Piece.line(lines[line].0, cut: LucidePart.cut(7, 0, 7 + (lines[line].1 + 1 - 7) * written[line], 24))
                }
            }
            .frame(width: size, height: size)
        } keyframes: { _ in
            KeyframeTrack(\.first) { MoveKeyframe(0); lift(1, 0.45 * total) }
            KeyframeTrack(\.second) { MoveKeyframe(0); LinearKeyframe(0, duration: 0.2 * total); lift(1, 0.45 * total) }
            KeyframeTrack(\.third) { MoveKeyframe(0); LinearKeyframe(0, duration: 0.4 * total); lift(1, 0.45 * total) }
        }
    }

    /// The folder flashes full at 30 % and drains (420 ms).
    private var folder: some View {
        let total = 0.42
        return LucideView(icon: .folder, size: size).keyframeAnimator(initialValue: Beat(), trigger: run) { glyph, now in
            glyph.background { LucidePart.Piece.folderBody.opacity(now.first) }
        } keyframes: { _ in
            KeyframeTrack(\.first) { LinearKeyframe(1, duration: 0.3 * total); LinearKeyframe(0, duration: 0.7 * total) }
        }
    }
}
