// An SF Symbol in one of its motions at a given progress (`SymbolMotion`): everything it draws is a function of `progress`, which
// the one that plays it moves from 0 to 1 (or between two states), and a still can hold at any value. It is Animatable, so a
// state's change with an animation (`SymbolMotion.animation`) moves it as the columns' slide moves the toggles. The symbol is
// drawn in the weight and size the app would draw it, and each piece in its own box (`SymbolPart`).

import SwiftUI

/// An SF Symbol in one of its motions at a given progress.
public struct SymbolMotionView: View, @preconcurrency Animatable {
    let symbol: String
    let alternate: String?
    let motion: SymbolMotion
    let size: CGFloat
    let weight: Font.Weight
    /// 0...1: how far a motion that plays once has played, or how far a state has changed.
    public var progress: Double

    /// `symbol`, with `alternate` (its filled variant, the check it becomes) where the motion uses one.
    public nonisolated init(symbol: String, alternate: String?, motion: SymbolMotion, size: CGFloat, weight: Font.Weight = .regular, progress: Double) {
        (self.symbol, self.alternate, self.motion, self.size, self.weight, self.progress) = (symbol, alternate, motion, size, weight, progress)
    }

    /// What `progress` is, so SwiftUI moves it between frames.
    public var animatableData: Double {
        get { progress }
        set { progress = newValue }
    }

    private var other: String { alternate ?? symbol }
    private var ink: CGRect { SymbolGeometry.ink(other) }

    private func part(_ name: String, _ clip: SymbolClip? = nil) -> SymbolPart { SymbolPart(name: name, size: size, weight: weight, clip: clip) }

    /// The curves of `view::motion::ease`.
    private static let easeOut = UnitCurve.bezier(startControlPoint: UnitPoint(x: 0.2, y: 0), endControlPoint: UnitPoint(x: 0, y: 1))
    private static let easeIn = UnitCurve.bezier(startControlPoint: UnitPoint(x: 0.3, y: 0), endControlPoint: UnitPoint(x: 1, y: 1))
    private static let easeInOut = UnitCurve.bezier(startControlPoint: UnitPoint(x: 0.4, y: 0), endControlPoint: UnitPoint(x: 0.2, y: 1))

    /// How far `t` is from `start` to `end`, 0...1, in a straight line (`timeline::span`).
    private func span(_ start: Double, _ end: Double) -> Double { min(max((progress - start) / (end - start), 0), 1) }

    /// A fill that grows to `GROWN_BY` of the run, decelerating, and lets go over the rest, accelerating (`pane_fill`).
    private var grown: Double { Self.easeOut.value(at: span(0, 0.45)) }
    private var strength: Double { 1 - Self.easeIn.value(at: span(0.45, 1)) }

    /// The symbol, its pieces laid as the motion has them at `progress`.
    public var body: some View {
        switch motion {
        case .quarter: part(symbol).rotationEffect(.degrees(90 * Self.easeOut.value(at: progress)))
        case .spin(let clockwise): part(symbol).rotationEffect(.degrees((clockwise ? 360 : -360) * Self.easeInOut.value(at: progress)))
        case .turn(let degrees): part(symbol).rotationEffect(.degrees(degrees * progress))
        case .copied: copied
        case .paneFill(let half): paneFill(half)
        case .pulse(let edge):
            ZStack {
                part(symbol)
                part(other, .grown(from: edge, grown, over: ink)).opacity(strength)
            }
        case .disc:
            ZStack {
                part(symbol)
                part(other, disc(grown)).opacity(strength)
            }
        case .written(let edge): part(symbol, .grown(from: edge, Self.easeOut.value(at: progress), over: SymbolGeometry.ink(symbol)))
        case .lines: lines
        case .around: part(symbol, SymbolClip(keeps: [.wedge(SymbolGeometry.clockCentre, start: -90, sweep: 360 * Self.easeOut.value(at: progress))]))
        case .needle: needle
        case .flash:
            ZStack {
                part(symbol)
                part(other).opacity(progress < 0.3 ? progress / 0.3 : 1 - (progress - 0.3) / 0.7)
            }
        case .strip(let edge): part(symbol).symbolFill(strip(edge))
        case .wipe(let edge):
            ZStack {
                part(symbol).opacity(progress >= 1 ? 0 : 1)
                part(other, .grown(from: edge, progress, over: ink))
            }
        case .grow:
            ZStack {
                part(symbol).opacity(progress >= 1 ? 0 : 1)
                part(other, disc(progress))
            }
        case .swap: swap
        }
    }

    /// The alternate's own disc, from its middle, wide enough at 1 to cover it.
    private func disc(_ share: Double) -> SymbolClip {
        SymbolClip(keeps: [.disc(CGPoint(x: ink.midX, y: ink.midY), radius: 0.7 * share)])
    }

    /// The front sheet fills (12 % of the run) and lets go as the check takes over (22 %), which holds until 86 % and gives the
    /// copy back; the check grows from 0.7 of its size (`copied`).
    private var copied: some View {
        let swap = span(0.12, 0.22), back = span(0.86, 1)
        let sheet = SymbolClip(keeps: [.rounded(SymbolGeometry.copySheet, corners: RectangleCornerRadii(
            topLeading: SymbolGeometry.copySheetCorner, bottomLeading: SymbolGeometry.copySheetCorner,
            bottomTrailing: SymbolGeometry.copySheetCorner, topTrailing: SymbolGeometry.copySheetCorner))])
        return ZStack {
            part(symbol).symbolFill(sheet, opacity: span(0, 0.12) * (1 - swap)).opacity(1 - swap + back)
            part(other).scaleEffect(0.7 + 0.3 * swap).opacity(swap - back)
        }
    }

    /// The half the new pane takes grows from the divider outward, drawn with the symbol's `.fill` variant, and lets go.
    private func paneFill(_ half: IconMotion.Half) -> some View {
        let middle = SymbolGeometry.Frame.middle, ink = SymbolGeometry.ink(other)
        let rect = half == .right
            ? CGRect(x: middle, y: -0.1, width: (ink.maxX - middle + 0.01) * grown, height: 1.2)
            : CGRect(x: -0.1, y: middle, width: 1.2, height: (ink.maxY - middle + 0.01) * grown)
        return ZStack {
            part(symbol)
            part(other, SymbolClip(keeps: [.rect(rect)])).opacity(strength)
        }
    }

    /// The strip between a frame's side and its divider, as full as `progress`, its outer corners the frame's.
    private func strip(_ edge: SymbolMotion.Edge) -> SymbolClip {
        typealias F = SymbolGeometry.Frame
        let corner = F.insideCorner, height = F.insideBottom - F.insideTop
        if edge == .trailing {
            let full = F.insideRight - F.rightDividerEnd
            return SymbolClip(keeps: [.rounded(
                CGRect(x: F.insideRight - full * progress, y: F.insideTop, width: full * progress, height: height),
                corners: RectangleCornerRadii(bottomTrailing: corner, topTrailing: corner))])
        }
        return SymbolClip(keeps: [.rounded(
            CGRect(x: F.insideLeft, y: F.insideTop, width: (F.leftDividerStart - F.insideLeft) * progress, height: height),
            corners: RectangleCornerRadii(topLeading: corner, bottomLeading: corner))])
    }

    /// The page's lines (`note.text`) written one after the other, the second a fifth of the run after the first (`TabGlyph`).
    private var lines: some View {
        let lines = SymbolGeometry.noteLines
        return ZStack {
            part(symbol, SymbolClip(keeps: [.rect(CGRect(x: -0.1, y: -0.1, width: 1.2, height: 1.2))], without: lines.map { .rect($0) }))
            ForEach(lines.indices, id: \.self) { index in
                let share = Self.easeOut.value(at: span(0.2 * Double(index), 0.2 * Double(index) + 0.45))
                part(symbol, SymbolClip(keeps: [.rect(CGRect(
                    x: lines[index].minX, y: lines[index].minY, width: lines[index].width * share + (share >= 1 ? 0.01 : 0),
                    height: lines[index].height))]))
            }
        }
    }

    /// The needle starts turned back 135 degrees, swings 6 % past its place at 70 % and settles (520 ms); the dial and its dots
    /// stay (`TabGlyph`'s gauge).
    private var needle: some View {
        typealias G = SymbolGeometry.Gauge
        let angle = progress < 0.7
            ? -135 + 143.1 * Self.easeOut.value(at: progress / 0.7)
            : 0.06 * 135 * (1 - Self.easeInOut.value(at: (progress - 0.7) / 0.3))
        let region = SymbolClip.Piece.tilted(center: G.needleCentre, size: G.needleSize, degrees: G.needleDegrees)
        return ZStack {
            part(symbol, SymbolClip(keeps: [.rect(CGRect(x: -0.1, y: -0.1, width: 1.2, height: 1.2))], without: [region]))
            part(symbol, SymbolClip(keeps: [region])).rotationEffect(.degrees(angle), anchor: UnitPoint(x: G.hub.x, y: G.hub.y))
        }
    }

    /// The symbol shrinks to 0.6 and fades over the first half, the alternate grows from 0.6 over the second (`IconSwap`).
    private var swap: some View {
        let away = Self.easeOut.value(at: span(0, 0.5)), arrive = Self.easeOut.value(at: span(0.5, 1))
        return ZStack {
            part(symbol).scaleEffect(1 - 0.4 * away).opacity(1 - away)
            part(other).scaleEffect(0.6 + 0.4 * arrive).opacity(arrive)
        }
    }
}
