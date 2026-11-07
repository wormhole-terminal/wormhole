// The control folder (WORMHOLE_DEVCTL=<dir>), off unless the variable is set. The caller writes `cmd.tmp` and
// renames it to `cmd`; the app reads and removes `cmd`, runs its lines and appends the results to `out.txt`.
// Event-driven: a dispatch source on the folder wakes the app when something is written, and nothing runs
// between commands (no timer, so an idle instance stays idle). Owner-only, like the GPUI app's devctl.

import Foundation

@MainActor
final class DevCtl {
    private let directory: URL
    private let handler: @MainActor (String) -> Void
    private var source: DispatchSourceFileSystemObject?

    init(directory: URL, handler: @escaping @MainActor (String) -> Void) {
        self.directory = directory
        self.handler = handler
    }

    func start() throws {
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let descriptor = open(directory.path, O_EVTONLY)
        guard descriptor >= 0 else { throw CocoaError(.fileReadNoPermission) }
        let source = DispatchSource.makeFileSystemObjectSource(fileDescriptor: descriptor, eventMask: .write, queue: .main)
        source.setEventHandler { [weak self] in MainActor.assumeIsolated { self?.drain() } }
        source.setCancelHandler { close(descriptor) }
        source.resume()
        self.source = source
        // A command written before the source existed.
        drain()
    }

    private func drain() {
        let command = directory.appendingPathComponent("cmd")
        guard let data = try? Data(contentsOf: command) else { return }
        try? FileManager.default.removeItem(at: command)
        for line in String(decoding: data, as: UTF8.self).split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if !trimmed.isEmpty { handler(trimmed) }
        }
    }

    /// Append to `out.txt` (results) or `frames.txt` (the timing lines).
    func append(_ text: String, to name: String = "out.txt") {
        let url = directory.appendingPathComponent(name)
        let line = text.hasSuffix("\n") ? text : text + "\n"
        if let handle = try? FileHandle(forWritingTo: url) {
            handle.seekToEndOfFile()
            handle.write(Data(line.utf8))
            try? handle.close()
        } else {
            try? Data(line.utf8).write(to: url, options: .atomic)
        }
    }
}
