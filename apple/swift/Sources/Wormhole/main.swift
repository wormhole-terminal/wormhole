// Wormhole, the Swift UI experiment's terminal: see README.md for what it is and is not.

import AppKit
import Foundation

let arguments = Array(CommandLine.arguments.dropFirst())
if arguments.contains("-h") || arguments.contains("--help") {
    print(Options.usage)
    exit(0)
}

do {
    let options = try Options.parse(arguments, environment: ProcessInfo.processInfo.environment)
    let dataDirectory = try Isolation.require()
    switch options.mode {
    case .renderPNG(let prefix, let frames, let every):
        try RenderPNG.run(options: options, prefix: prefix, frames: frames, every: every)
    case .app:
        let app = NSApplication.shared
        app.setActivationPolicy(options.background ? .accessory : .regular)
        let controller = AppController(options: options, dataDirectory: dataDirectory)
        app.delegate = controller
        app.run()
    }
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(2)
}
