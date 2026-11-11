// The settings pages read and write other programs' files (agent hooks, the terminals to import from, the `wh`
// command). The core writes agent hooks only under WORMHOLE_AGENT_HOME, reads imports from WORMHOLE_IMPORT_HOME and
// installs `wh` only in WORMHOLE_CLI_DIR when they are set, so an instance of the experiment keeps all three inside its
// own data folder unless the caller pointed them somewhere else (`WORMHOLE_CLI_DIR=/usr/local/bin` puts `wh` on
// the PATH of every terminal): nothing of the person's real home is ever touched from here.

import Foundation

extension Isolation {
    static func confineHomes(to dataDirectory: URL, environment: [String: String] = ProcessInfo.processInfo.environment) {
        for (name, folder) in [("WORMHOLE_AGENT_HOME", "agent-home"), ("WORMHOLE_IMPORT_HOME", "import-home"), ("WORMHOLE_CLI_DIR", "cli-bin")]
        where environment[name]?.isEmpty ?? true {
            let url = dataDirectory.appendingPathComponent(folder)
            try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            setenv(name, url.path, 1)
        }
    }
}
