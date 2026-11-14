// Wormhole, the Swift UI experiment's terminal: see README.md for what it is and is not.

import AppKit
import Foundation
#if WITH_UNIFFI
    import HookBridge
#endif

#if WITH_UNIFFI
    // The agents' hook scripts and a terminal's own `wh` run this executable (`$WORMHOLE_BIN hook …`): a command line is
    // answered before anything of an app is set up. Which first word is a command line is the core's to say, as GPUI's
    // `main.rs:63` asks it (`ipc::command`): `wh --help` and `wh -h` are the control line's usage there, not this app's.
    // A word without a dash is always the core's (a verb, or an unknown command it refuses with exit 2); one with a dash
    // that the core does not know is one of this app's own launch options (`--shell`, `--background`), which GPUI does
    // not have, or a mistake `Options.parse` refuses with exit 2 as well (`--json` or `--version` first: never a launch).
    let isCommandLine = CommandLine.arguments.dropFirst().first.map { !$0.hasPrefix("-") || HookBridge.isCommandLine() } ?? false
    if isCommandLine, let code = HookBridge.exitCodeIfCommandLine() { exit(code) }
#endif

let arguments = Array(CommandLine.arguments.dropFirst())
if arguments.contains("-h") || arguments.contains("--help") {
    print(Options.usage)
    exit(0)
}

do {
    let options = try Options.parse(arguments, environment: ProcessInfo.processInfo.environment)
    // `--shell` is `WORMHOLE_SHELL` given on the command line and means exactly what it means: the core reads the variable
    // where it decides what a terminal runs and is called (core `terminal/process.rs:97`, `:166`), and it replaces every
    // choice, a terminal's own shell too, so a `--shell /bin/cat` instance never runs a real shell. GPUI is given a forced
    // shell as that variable only (`scripts/lib.sh:140`). Set here, before anything else runs: no thread reads it yet.
    if let program = options.program { setenv("WORMHOLE_SHELL", program, 1) }  // text: an environment variable name
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
