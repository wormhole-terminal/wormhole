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
    public static let rest = StatusPose(flareReach: 0, flareStrength: 0, dx: 0)
    public init(flareReach: Double, flareStrength: Double, dx: Double) { (self.flareReach, self.flareStrength, self.dx) = (flareReach, flareStrength, dx) }
}

public struct RowMotionTables: Sendable {
    /// How long the change from the tile state at `from` to the one at `to` (their places in the core's urgency order) plays under
    /// the `motion.status` choice, and where it stands `t` of the way through.
    public var statusSeconds: @Sendable (_ variant: String, _ from: Int, _ to: Int) -> Double = { _, _, _ in 0 }
    public var statusPose: @Sendable (_ variant: String, _ from: Int, _ to: Int, _ t: Double) -> StatusPose = { _, _, _, _ in .rest }

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
