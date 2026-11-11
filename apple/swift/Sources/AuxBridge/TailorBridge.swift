// `TailorWalking` over core's walk (`OnboardingTailor`): records mapped to the values the Look step draws.

import AuxModel
import Foundation
import WormholeBindings

@MainActor
final class FFITailor: TailorWalking {
    private let walk: OnboardingTailor

    init(panelOpen: Bool, dataDir: URL?) throws { walk = try OnboardingTailor(panelOpen: panelOpen, dataDir: dataDir?.path) }

    func view(width: Double) -> TailorView {
        let record = walk.view(width: Float(width))
        let stage: TailorView.Stage =
            switch record.stage {
            case .asking: .asking
            case .summary: .summary
            case .settled: .settled
            }
        return TailorView(
            stage: stage, heading: record.heading, question: record.question, counter: record.counter, detail: record.detail,
            summaryTitle: record.summaryTitle, settledText: record.settledText, backLabel: record.back,
            startOverLabel: record.startOver, looksGoodLabel: record.looksGood, announcement: record.announcement,
            answers: record.answers.map {
                TailorAnswer(label: $0.label, detail: $0.detail, picked: $0.picked, sketch: $0.sketch.map(\.block))
            },
            summary: record.summary.map { ($0.question, $0.answer) }, canBack: record.canBack, panelOpen: record.panelOpen)
    }

    func act(_ act: TailorAct) -> Bool {
        let record: TailorActRecord =
            switch act {
            case .answer(let n): .answer(n: UInt32(n))
            case .back: .back
            case .startOver: .startOver
            case .settle: .settle
            }
        return walk.act(act: record)
    }
}

extension SketchPiece {
    var block: SketchBlock {
        let fill: SketchBlock.Fill =
            switch self.fill {
            case .sidebar: .sidebar
            case .panelRight: .panelRight
            case .rule: .rule
            case .selectedRow: .selectedRow
            case .faint: .faint
            case .muted: .muted
            case .chip: .chip
            case .primary: .primary
            case .tab: .tab
            case .minimap: .minimap
            }
        let corner: SketchBlock.Corner =
            switch self.corner {
            case .square: .square
            case .soft: .soft
            case .round: .round
            }
        return SketchBlock(x: Double(x), y: Double(y), w: Double(w), h: Double(h), fill: fill, corner: corner)
    }
}

/// The window sketch of preset `index` (0 Zen, 1 Balanced, 2 Everything): the Studio gallery's specimen.
public enum WindowSketches {
    /// The blocks to paint.
    public static func blocks(preset index: Int, width: Double) -> [SketchBlock] {
        windowSketch(preset: UInt32(index), width: Float(width)).map(\.block)
    }
}
