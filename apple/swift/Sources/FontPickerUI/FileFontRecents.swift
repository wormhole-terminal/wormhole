// The recents in a file of the instance's data folder (`font-recents.json`, the folder `WORMHOLE_DATA_DIR` names), so
// one instance never reads or writes another's, nor anything outside its folder. Without a folder (tests, previews)
// they live as long as the process.

import FontPickerModel
import Foundation
import Synchronization

public final class FileFontRecents: FontRecents {
    private let bySlot: Mutex<[String: [String]]>
    private let file: URL?

    public init(folder: URL? = ProcessInfo.processInfo.environment["WORMHOLE_DATA_DIR"].map { URL(fileURLWithPath: $0) }) {  // text: an environment variable
        file = folder?.appendingPathComponent("font-recents.json")
        let saved = file.flatMap { try? Data(contentsOf: $0) }.flatMap { try? JSONDecoder().decode([String: [String]].self, from: $0) }
        bySlot = Mutex(saved ?? [:])
    }

    public func families(for slot: String) -> [String] { bySlot.withLock { $0[slot] ?? [] } }

    public func store(_ families: [String], for slot: String) {
        let data = bySlot.withLock {
            $0[slot] = families
            return try? JSONEncoder().encode($0)
        }
        if let file, let data { try? data.write(to: file, options: .atomic) }
    }
}
