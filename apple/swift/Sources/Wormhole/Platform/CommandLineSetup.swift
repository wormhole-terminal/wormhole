// The `wh` command in every terminal of the app: at launch core writes a small script into `<data folder>/bin` that runs
// this executable (`cli_tool::keep_private_current`), and each terminal the core starts has that folder first on its
// PATH. It must exist before the first terminal starts, since the PATH is read when a terminal is spawned.

#if WITH_UNIFFI
    import Foundation
    import WormholeBindings

    enum CommandLineSetup {
        static func keepCurrent() {
            guard let binary = Bundle.main.executablePath else { return }
            do {
                _ = try cliToolKeepCurrent(binaryPath: binary)
            } catch {
                FileHandle.standardError.write(Data("wh: terminals start without the command (\(error))\n".utf8))  // text: a log line
            }
        }
    }
#endif
