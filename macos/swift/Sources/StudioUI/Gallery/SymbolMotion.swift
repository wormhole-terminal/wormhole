// The ways an SF Symbol could answer a click, for the "SF Symbols" specimen: each changes the symbol's own drawing (one
// symbol becomes its variant, the strokes draw off and on, the layers light in turn) and none moves the symbol as a whole,
// the rule the icons' motion is held to (`IconMotion`). The Animate icons setting and Reduce Motion cut them all: the symbol
// still changes, at once.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

enum SymbolMotion: Equatable, Sendable {
    /// How a symbol is replaced by its variant (`ReplaceSymbolEffect`).
    enum Replace: Hashable, Sendable { case magic, downUp, upUp, offUp }
    /// What draws as a whole, what draws layer by layer, and what draws stroke by stroke (`DrawOnSymbolEffect`).
    enum Drawing: Hashable, Sendable { case whole, byLayer, individually }

    /// The symbol becomes its variant, and a second click turns it back.
    case replace(Replace)
    /// The strokes draw off, then draw on again.
    case draw(Drawing)
    /// The system's rotate effect, which turns the layers a symbol marks as turning.
    case rotate
    /// The layers light one after the other, as far as the symbol has layers that do.
    case variableColor
    /// The motion the app gives the same icon today (`IconMotion`), for comparison.
    case today(IconMotion)

    var title: String {
        switch self {
        case .replace(.magic): "magic replace"
        case .replace(.downUp): "replace, down then up"
        case .replace(.upUp): "replace, up then up"
        case .replace(.offUp): "replace, off then up"
        case .draw(.whole): "draw, whole symbol"
        case .draw(.byLayer): "draw, layer by layer"
        case .draw(.individually): "draw, stroke by stroke"
        case .rotate: "system rotate"
        case .variableColor: "variable colour"
        case .today(.quarter): "today: a quarter turn"
        case .today(.spin): "today: a spin"
        case .today(.copied): "today: copy becomes a check"
        case .today(.newPane): "today: the new pane's half fills"
        }
    }

    /// The same, a few words shorter, for the line under a row.
    var short: String {
        switch self {
        case .replace(.magic): "magic replace"
        case .replace(.downUp): "replace down-up"
        case .replace(.upUp): "replace up-up"
        case .replace(.offUp): "replace off-up"
        case .draw(.whole): "draw"
        case .draw(.byLayer): "draw by layer"
        case .draw(.individually): "draw by stroke"
        case .rotate: "rotate"
        case .variableColor: "variable colour"
        case .today: "today's motion"
        }
    }

    var replaces: Bool {
        if case .replace = self { true } else { false }
    }
}

/// One symbol that plays `motion` when it is clicked, and again at each click.
struct SymbolPlayer: View {
    let symbol: String
    let alternate: String?
    let motion: SymbolMotion
    let size: CGFloat
    let weight: Font.Weight
    let letter: String

    @State private var showsAlternate = false
    @State private var drawn = true
    @State private var run = 0
    @Environment(\.iconAnimations) private var animations
    @Environment(\.reduceMotion) private var reduceMotion

    private var plays: Bool { IconMotion.plays(animations: animations, reduceMotion: reduceMotion) }

    var body: some View {
        Button(action: play) {
            glyph
                .font(.system(size: size, weight: weight))
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(motion.title)
        .accessibilityLabel("\(letter): \(motion.title)")
    }

    @ViewBuilder private var glyph: some View {
        switch motion {
        case .replace(let style):
            Image(systemName: showsAlternate ? alternate ?? symbol : symbol)
                .contentTransition(plays ? Self.transition(style) : .identity)
        case .draw(let drawing):
            ZStack {
                if drawn { Image(systemName: symbol).transition(Self.drawing(drawing)) }
            }
        case .rotate:
            Image(systemName: symbol).symbolEffect(.rotate, options: .nonRepeating, value: run)
        case .variableColor:
            Image(systemName: symbol)
                .symbolEffect(.variableColor.iterative.dimInactiveLayers, options: .nonRepeating, value: run)
        case .today(let own):
            Image(systemName: symbol).iconMotion(own, run: run)
        }
    }

    private func play() {
        switch motion {
        case .replace:
            withAnimation(plays ? .default : nil) { showsAlternate.toggle() }
        case .draw:
            guard drawn, plays else { return }
            withAnimation { drawn = false }
            Task {
                // one-shot: the strokes finish drawing off before they draw on again
                try? await Task.sleep(for: .milliseconds(600))
                withAnimation { drawn = true }
            }
        case .rotate, .variableColor, .today:
            if plays { run += 1 }
        }
    }

    private static func transition(_ style: SymbolMotion.Replace) -> ContentTransition {
        switch style {
        case .magic: .symbolEffect(.replace.magic(fallback: .downUp))
        case .downUp: .symbolEffect(.replace.downUp)
        case .upUp: .symbolEffect(.replace.upUp)
        case .offUp: .symbolEffect(.replace.offUp)
        }
    }

    private static func drawing(_ drawing: SymbolMotion.Drawing) -> SymbolEffectTransition {
        switch drawing {
        case .whole: .symbolEffect(.drawOn.wholeSymbol)
        case .byLayer: .symbolEffect(.drawOn.byLayer)
        case .individually: .symbolEffect(.drawOn.individually)
        }
    }
}
