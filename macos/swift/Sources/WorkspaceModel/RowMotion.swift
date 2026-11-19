// What a row of the list does as it comes, goes, or its group folds or opens: the core's per-variant tables
// (`view::motion::pose`), by the name the settings spell the choice by (`motion.insert`, `motion.fold`). The core is
// installed at launch (`WorkspaceBridge`); without it (previews, tests) there are no tables and the list keeps its plain fades.

import Foundation
import Synchronization

/// How a row stands at a moment of its motion.
public struct RowPose: Sendable, Equatable {
    public var opacity, dx, dy, clip: Double
    public static let rest = RowPose(opacity: 1, dx: 0, dy: 0, clip: 1)
    public init(opacity: Double, dx: Double, dy: Double, clip: Double) { (self.opacity, self.dx, self.dy, self.clip) = (opacity, dx, dy, clip) }
}

/// How long a motion takes, and what makes room for the rows around.
public struct RowMotionPlan: Sendable, Equatable {
    /// A row coming: the time it takes. A row going: the time it stays drawn as a pose (0: it is cut).
    public var seconds: Double
    /// The rows around move on this spring (stiffness, damping); nil: the row's height follows its pose's `clip`.
    public var roomSpring: (stiffness: Double, damping: Double)?

    public init(seconds: Double, roomSpring: (stiffness: Double, damping: Double)?) { (self.seconds, self.roomSpring) = (seconds, roomSpring) }
    public static func == (a: Self, b: Self) -> Bool { a.seconds == b.seconds && a.roomSpring?.stiffness == b.roomSpring?.stiffness && a.roomSpring?.damping == b.roomSpring?.damping }
}

/// How a tile and its row stand at a moment of a status change (`pose::status`): the parts that matter to Flare and Nudge.
public struct StatusPose: Sendable, Equatable {
    /// How far outside the tile's edge the flare's ring reaches (points) and how strong it is.
    public var flareReach, flareStrength: Double
    /// How far the row is moved sideways (points).
    public var dx: Double
    /// The tile's own parts, for a picture drawn from the pose alone (Settings → Motion, the Studio's gallery): how much of the
    /// old status's light and badge still shows, of the new one's light or edge (`ring`), the new badge's opacity and size, the
    /// card's wash and how far the second line has rolled to the new words.
    public var leaving = 0.0, ring = 1.0, badgeOpacity = 1.0, badgeScale = 1.0, wash = 0.0, roll = 1.0
    public static let rest = StatusPose(flareReach: 0, flareStrength: 0, dx: 0)
    public init(flareReach: Double, flareStrength: Double, dx: Double) { (self.flareReach, self.flareStrength, self.dx) = (flareReach, flareStrength, dx) }
}

public struct RowMotionTables: Sendable {
    /// How long the change from the tile state at `from` to the one at `to` (their places in the core's urgency order) plays under
    /// the `motion.status` choice, and where it stands `t` of the way through.
    public var statusSeconds: @Sendable (_ variant: String, _ from: Int, _ to: Int) -> Double = { _, _, _ in 0 }
    public var statusPose: @Sendable (_ variant: String, _ from: Int, _ to: Int, _ t: Double) -> StatusPose = { _, _, _, _ in .rest }

    /// Settings → Motion's scenes (`motion_scenes.rs`): the room a coming or going row leaves the rows around it and a group's
    /// rows leave the lines below (0...1, `seconds` after the motion began), a folded group's summary, and a side column's
    /// length and pose (`motion.column`). Without the core everything stands at its rest.
    public var insertRoom: @Sendable (_ variant: String, _ entering: Bool, _ seconds: Double) -> Double = { _, entering, _ in entering ? 1 : 0 }
    public var foldRoom: @Sendable (_ variant: String, _ entering: Bool, _ rows: Int, _ seconds: Double) -> Double = { _, entering, _, _ in entering ? 1 : 0 }
    public var foldSummary: @Sendable (_ variant: String, _ rows: Int, _ seconds: Double) -> Double = { _, _, _ in 1 }
    public var columnSeconds: @Sendable (_ variant: String, _ opening: Bool) -> Double = { _, _ in 0 }
    public var columnPose: @Sendable (_ variant: String, _ opening: Bool, _ seconds: Double) -> (shown: Double, content: Double) = { _, opening, _ in (opening ? 1 : 0, 1) }

    /// How long a split's bracket takes to draw down its rows, and how much of a stretch drawn from `from` to `to` of that time shows
    /// `t` of the way through (`view::motion::drag`). Without the core the bracket is there at once.
    public var bracketSeconds: @Sendable () -> Double = { 0 }
    public var bracketShare: @Sendable (_ t: Double, _ from: Double, _ to: Double) -> Double = { _, _, _ in 1 }
    /// How far each line (by its index) steps aside to open a gap before line `slot` for the dragged lines `run` (core
    /// `room::make_room`; down is positive). Without the core nothing moves.
    public var roomPushes: @Sendable (_ lines: [(top: Double, height: Double)], _ run: Range<Int>, _ slot: Int) -> [Int: Double] = { _, _, _ in [:] }

    public var insertPlan: @Sendable (_ variant: String, _ entering: Bool) -> RowMotionPlan
    public var insertPose: @Sendable (_ variant: String, _ entering: Bool, _ t: Double) -> RowPose
    public var foldPlan: @Sendable (_ variant: String, _ entering: Bool, _ rows: Int) -> RowMotionPlan
    public var foldPose: @Sendable (_ variant: String, _ entering: Bool, _ t: Double, _ index: Int, _ rows: Int) -> RowPose

    public init(
        insertPlan: @escaping @Sendable (String, Bool) -> RowMotionPlan, insertPose: @escaping @Sendable (String, Bool, Double) -> RowPose,
        foldPlan: @escaping @Sendable (String, Bool, Int) -> RowMotionPlan, foldPose: @escaping @Sendable (String, Bool, Double, Int, Int) -> RowPose
    ) { (self.insertPlan, self.insertPose, self.foldPlan, self.foldPose) = (insertPlan, insertPose, foldPlan, foldPose) }
}

public enum RowMotion {
    private static let installed = Mutex<RowMotionTables?>(nil)
    public static func install(_ tables: RowMotionTables?) { installed.withLock { $0 = tables } }
    /// The core's tables, nil where none is installed.
    public static var tables: RowMotionTables? { installed.withLock { $0 } }
}
