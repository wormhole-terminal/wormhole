// The core's frame records and Swift's `WindowFrame`, field for field.

import WindowsModel
import WormholeBindings

extension WindowsModel.WindowFrame {
    init(_ record: FrameRecord) {
        let kind: WindowsModel.FrameKind =
            switch record.kind {
            case .windowed: .windowed
            case .maximized: .maximized
            case .fullscreen: .fullscreen
            }
        self.init(
            kind: kind, x: Double(record.x), y: Double(record.y), width: Double(record.width), height: Double(record.height),
            display: record.display, minimised: record.minimised)
    }

    var record: FrameRecord {
        let kind: WormholeBindings.FrameKind =
            switch self.kind {
            case .windowed: .windowed
            case .maximized: .maximized
            case .fullscreen: .fullscreen
            }
        return FrameRecord(
            kind: kind, x: Float(x), y: Float(y), width: Float(width), height: Float(height), display: display, minimised: minimised)
    }
}

extension DisplayArea {
    var record: DisplayRecord { DisplayRecord(id: id, x: Float(x), y: Float(y), width: Float(width), height: Float(height)) }
}
