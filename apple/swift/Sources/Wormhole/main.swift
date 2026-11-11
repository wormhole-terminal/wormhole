// Wormhole, the Swift UI experiment's terminal: see README.md for what it is and is not.

import AppKit
import Foundation
#if WITH_UNIFFI
    import HookBridge
#endif

#if WITH_UNIFFI
    // The agents' hook scripts and a terminal's own `wh` run this executable (`$WORMHOLE_BIN hook …`): a command line is
    // answered before anything of an app is set up. The app's own options (`--shell`, `--background`) start with a dash;
    // the core would call them unknown commands.
    if let first = CommandLine.arguments.dropFirst().first, !first.hasPrefix("-"), let code = HookBridge.exitCodeIfCommandLine() {
        exit(code)
    }
#endif

let arguments = Array(CommandLine.arguments.dropFirst())
if arguments.contains("-h") || arguments.contains("--help") {
    print(Options.usage)
    exit(0)
}

do {
    let options = try Options.parse(arguments, environment: ProcessInfo.processInfo.environment)
    let dataDirectory = try Isolation.require()
    Isolation.confineHomes(to: dataDirectory)
    switch options.mode {
    case .renderPNG(let prefix, let frames, let every):
        try RenderPNG.run(options: options, prefix: prefix, frames: frames, every: every)
    case .app:
        let app = NSApplication.shared
        app.setActivationPolicy(options.background ? .accessory : .regular)
        let controller = try AppController(options: options, dataDirectory: dataDirectory)
        app.delegate = controller
        app.run()
    }
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(2)
}
