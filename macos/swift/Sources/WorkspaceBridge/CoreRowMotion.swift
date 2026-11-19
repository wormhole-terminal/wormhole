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
            var status = StatusPose(flareReach: Double(pose.flareReach), flareStrength: Double(pose.flareStrength), dx: Double(pose.dx))
            (status.leaving, status.badgeOpacity, status.badgeScale) = (Double(pose.leaving), Double(pose.badgeOpacity), Double(pose.badgeScale))
            (status.wash, status.roll, status.ring) = (Double(pose.wash), Double(pose.roll), Double(pose.ring))
            return status
        }
        tables.insertRoom = { Double(insertRoomAt(variant: $0, entering: $1, elapsedMs: Float($2 * 1000))) }
        tables.foldRoom = { Double(foldRoomAt(variant: $0, entering: $1, rows: UInt32($2), elapsedMs: Float($3 * 1000))) }
        tables.foldSummary = { Double(foldSummaryOpacity(variant: $0, rows: UInt32($1), elapsedMs: Float($2 * 1000))) }
        tables.bracketSeconds = { Double(bracketDrawMs()) / 1000 }
        tables.bracketShare = { Double(bracketShare(t: Float($0), from: Float($1), to: Float($2))) }
        tables.roomPushes = { lines, run, slot in
            let records = lines.map { RoomLine(top: Float($0.top), height: Float($0.height)) }
            let pushes = roomPushes(lines: records, runStart: UInt32(run.lowerBound), runEnd: UInt32(run.upperBound), slot: UInt32(slot))
            return Dictionary(pushes.map { (Int($0.line), Double($0.by)) }, uniquingKeysWith: { first, _ in first })
        }
        tables.columnSeconds = { Double(columnMotionDuration(variant: $0, opening: $1)) / 1000 }
        tables.columnPose = {
            let pose = columnMotionPose(variant: $0, opening: $1, elapsedMs: Float($2 * 1000))
            return (Double(pose.shown), Double(pose.content))
        }
        return tables
    }()

    private static let rows = RowMotionTables(
        insertPlan: { plan(insertRowMotion(variant: $0, entering: $1)) },
        insertPose: { pose(insertRowPose(variant: $0, entering: $1, t: Float($2))) },
        foldPlan: { plan(foldRowMotion(variant: $0, entering: $1, rows: UInt32($2))) },
        foldPose: { pose(foldRowPose(variant: $0, entering: $1, t: Float($2), index: UInt32($3), rows: UInt32($4))) })
}
