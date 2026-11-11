import Foundation
import Testing

@testable import FontPickerUI

@Suite struct FileFontRecentsTests {
    @Test func recentsLiveInTheGivenFolderAndComeBack() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("Wormhole-swift-recents-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        FileFontRecents(folder: folder).store(["Menlo", "Monaco"], for: "terminal")
        #expect(FileFontRecents(folder: folder).families(for: "terminal") == ["Menlo", "Monaco"])
        #expect(FileManager.default.fileExists(atPath: folder.appendingPathComponent("font-recents.json").path))
    }
}
