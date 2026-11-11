// The sidebar's row motion tables asked of the core (`insertRowMotion`, `foldRowPose`, ...: `view::motion::pose`).

import WorkspaceModel
import WormholeBindings

enum CoreRowMotion {
    static func pose(_ record: RowPoseRecord) -> RowPose {
        RowPose(opacity: Double(record.opacity), dx: Double(record.dx), dy: Double(record.dy), clip: Double(record.clip))
    }

    static func plan(_ record: RowMotionRecord) -> RowMotionPlan {
        let spring = record.roomSpring.flatMap { $0.count == 2 ? (stiffness: Double($0[0]), damping: Double($0[1])) : nil }
        return RowMotionPlan(seconds: Double(record.durationMs) / 1000, roomSpring: spring)
    }

    static let tables: RowMotionTables = {
        var tables = rows
        tables.statusSeconds = { Double(statusMotionDuration(variant: $0, from: UInt8($1), to: UInt8($2))) / 1000 }
        tables.statusPose = {
            let pose = statusMotionPose(variant: $0, from: UInt8($1), to: UInt8($2), t: Float($3))
            return StatusPose(flareReach: Double(pose.flareReach), flareStrength: Double(pose.flareStrength), dx: Double(pose.dx))
        }
        return tables
    }()

    private static let rows = RowMotionTables(
        insertPlan: { plan(insertRowMotion(variant: $0, entering: $1)) },
        insertPose: { pose(insertRowPose(variant: $0, entering: $1, t: Float($2))) },
        foldPlan: { plan(foldRowMotion(variant: $0, entering: $1, rows: UInt32($2))) },
        foldPose: { pose(foldRowPose(variant: $0, entering: $1, t: Float($2), index: UInt32($3), rows: UInt32($4))) })
}
