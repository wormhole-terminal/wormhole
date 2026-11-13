// Motion's scene (`motion_preview/mod.rs`): every option's picture at once, four cells across the strip (two rows of two
// where four have no room), each looping through the thing its choice is about and named under it by the row it belongs
// to. A cell shows its own choice, so a pick changes only its cell and starts that cell's loop again; the cell whose option
// changed last wears the ring. A scene is a function of the loop's time `t` (0..<1) drawn from the core's poses
// (`RowMotion.tables`), the ones the window itself draws; `t = 0` is the picture before anything moves, which Reduce
// Motion holds.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

/// What a cell is a preview of: the key of its choice.
enum MotionKind: String, CaseIterable {
    case insert = "motion.insert", fold = "motion.fold", status = "motion.status", column = "motion.column"

    /// The id of the row the scene belongs to, whose label names the cell.
    var entry: String {
        switch self {
        case .insert: "motion-insert"
        case .fold: "motion-fold"
        case .status: "motion-status"
        case .column: "motion-column"
        }
    }
}

struct MotionScene: View {
    let model: SettingsModel
    @State private var touched = MotionKind.insert
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    /// The narrowest a cell gets for its rows to read (`CELL_MIN_W`).
    private static let cellMinWidth: CGFloat = 152

    var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .top, spacing: Space._2) {
                ForEach(MotionKind.allCases, id: \.self) { cell($0).frame(minWidth: Self.cellMinWidth) }
            }
            VStack(spacing: Space._2) {
                HStack(alignment: .top, spacing: Space._2) { cell(.insert); cell(.fold) }
                HStack(alignment: .top, spacing: Space._2) { cell(.status); cell(.column) }
            }
        }
    }

    private func cell(_ kind: MotionKind) -> some View {
        MotionCell(
            kind: kind, variant: model.value(kind.rawValue).string ?? "classic", // text: the choice every option list starts with
            label: model.schema.entry(kind.entry)?.entry.label ?? "", tokens: PreviewTheme.tokens(model), isTouched: touched == kind,
            isStill: reduceMotion, onChanged: { touched = kind })
    }
}

private struct MotionCell: View {
    let kind: MotionKind
    let variant: String
    let label: String
    let tokens: ThemeTokens
    let isTouched: Bool
    let isStill: Bool
    let onChanged: () -> Void
    /// When this cell's loop began: now, and again whenever its option changes.
    @State private var began = Date()

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        VStack(alignment: .leading, spacing: Space._2) {
            TimelineView(.animation(paused: isStill)) { timeline in
                stage(at: isStill ? 0 : MotionLoop.time(from: began, to: timeline.date))
            }
            .clipShape(shape)
            .overlay { if isTouched { shape.strokeBorder(tokens.primary, lineWidth: 2) } }
            Text(label).textRole(.caption).lineLimit(1).foregroundStyle(isTouched ? tokens.primary : tokens.textMuted)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: variant) {
            began = Date()
            onChanged()
        }
    }

    @ViewBuilder private func stage(at t: Double) -> some View {
        switch kind {
        case .insert: MotionInsertScene(tokens: tokens, variant: variant, t: t)
        case .fold: MotionFoldScene(tokens: tokens, variant: variant, t: t)
        case .status: MotionStatusScene(tokens: tokens, variant: variant, t: t)
        case .column: MotionColumnScene(tokens: tokens, variant: variant, t: t)
        }
    }
}

/// The loop every scene shares (`LOOP`): four seconds, long enough to rest before and after each change.
enum MotionLoop {
    static let seconds = 4.0

    /// How far through its loop a cell that began at `began` is at `now`: 0 up to 1.
    static func time(from began: Date, to now: Date) -> Double {
        let loops = now.timeIntervalSince(began) / seconds
        return max(0, loops - loops.rounded(.down))
    }

    /// The seconds since the share `start` of the loop, at `t` of it: nothing before it.
    static func since(_ t: Double, _ start: Double) -> Double { max(0, t - start) * seconds }

    /// How far `elapsed` is through a motion `total` seconds long (one of none is over at once), as the core's poses take it.
    static func progress(_ elapsed: Double, of total: Double) -> Double { total > 0 ? min(1, elapsed / total) : 1 }
}
