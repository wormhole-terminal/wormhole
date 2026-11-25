// The settings pages read and write other programs' files (agent hooks, the terminals to import from, the `wh`
// command). The core writes agent hooks only under WORMHOLE_AGENT_HOME, reads imports from WORMHOLE_IMPORT_HOME and
// installs `wh` only in WORMHOLE_CLI_DIR when they are set. A dev or test instance keeps all three inside its own data
// folder unless the caller pointed them somewhere else (`WORMHOLE_CLI_DIR=/usr/local/bin` puts `wh` on the PATH of every
// terminal): nothing of the person's real home is touched from there. The beta is a real app, so its agent and import
// homes are the person's own (agent status and the Usage tab need the real `~/.claude`); only `wh` stays in a folder of
// its own, so installing it never replaces the 1.x app's.

import Foundation

extension Isolation {
    /// The homes a data folder keeps apart from the person's, as (variable, folder in the data folder): all three, except in
    /// the beta's folder, which keeps `wh` apart only.
    static func confinedHomes(for dataDirectory: URL) -> [(variable: String, folder: String)] {
        let homes = [("WORMHOLE_AGENT_HOME", "agent-home"), ("WORMHOLE_IMPORT_HOME", "import-home"), ("WORMHOLE_CLI_DIR", "cli-bin")]
        return dataDirectory.lastPathComponent == betaFolderName ? homes.filter { $0.0 == "WORMHOLE_CLI_DIR" } : homes
    }

    static func confineHomes(to dataDirectory: URL, environment: [String: String] = ProcessInfo.processInfo.environment) {
        for (name, folder) in confinedHomes(for: dataDirectory) where environment[name]?.isEmpty ?? true {
            let url = dataDirectory.appendingPathComponent(folder)
            try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            setenv(name, url.path, 1)
        }
    }
}
