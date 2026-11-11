// A picture of the folder, decoded for the viewer (`features/files/viewer/image.rs`): repository bytes are hostile,
// so the declared size is read first and an absurd one refused, and what is drawn is a thumbnail of the first frame,
// made once and off the main thread.

import CoreGraphics
import Foundation
import ImageIO

enum FilePicture: Sendable {
    case shown(CGImage)
    case tooLarge
    case unreadable

    /// The longest side drawn, and the most pixels a file may declare before it is refused (a small file can
    /// declare a bitmap of gigabytes).
    static let maxSide = 4_096
    static let maxPixels = 100_000_000

    static func decode(_ data: Data) -> FilePicture {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
            let facts = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
            let width = facts[kCGImagePropertyPixelWidth] as? Int, let height = facts[kCGImagePropertyPixelHeight] as? Int
        else { return .unreadable }
        guard width > 0, height > 0, width <= maxPixels / height else { return .tooLarge }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: maxSide,
        ]
        return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary).map(FilePicture.shown) ?? .unreadable
    }
}
