// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `stall <ms>`: the main thread does nothing for that long, as a busy Mac's main thread is held by other work
// while the Rust threads (the hook socket, the look at each terminal's process) go on. A test seam for the races between the two: it
// reaches only a test instance, which is the only app with a control folder. Replies `stall <ms>` when it is over.

import Foundation

extension AppController {
    /// False when `command` is not `stall`.
    func runStall(command: String, argument: String) -> Bool {
        guard command == "stall" else { return false }
        guard let milliseconds = Int(argument), (0...10_000).contains(milliseconds) else {
            reply("error: stall <ms>, up to 10000")
            return true
        }
        Thread.sleep(forTimeInterval: Double(milliseconds) / 1000)  // why: holding the main thread is the point of this command
        reply("stall \(milliseconds)")
        return true
    }
}
