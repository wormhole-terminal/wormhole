// The settings pages read and write other programs' files (agent hooks, the terminals to import from). The core writes
// agent hooks only under WORMHOLE_AGENT_HOME and reads imports from WORMHOLE_IMPORT_HOME when they are set, so an
// instance of the experiment keeps both inside its own data folder unless the caller pointed them somewhere else:
// nothing of the person's real home is ever touched from here.

import Foundation

extension Isolation {
    static func confineHomes(to dataDirectory: URL, environment: [String: String] = ProcessInfo.processInfo.environment) {
        for (name, folder) in [("WORMHOLE_AGENT_HOME", "agent-home"), ("WORMHOLE_IMPORT_HOME", "import-home")]
        where environment[name]?.isEmpty ?? true {
            let url = dataDirectory.appendingPathComponent(folder)
            try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            setenv(name, url.path, 1)
        }
    }
}
