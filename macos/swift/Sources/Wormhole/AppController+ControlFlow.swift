// The control folder's batch commands (`WORMHOLE_DEVCTL`), GPUI's `devctl/flow.rs` and `control.rs`: `delay <ms>` holds the
// rest of the batch, `sync <token>` says a script's lines before it have all run, and `control [--from <tab>] <words…>` sends
// a request as `Wormhole <words…>` does, from inside a tab when `--from` names one.

import Foundation
#if WITH_UNIFFI
    import HookBridge
#endif

extension AppController {
    /// False when `command` is not one of these.
    func runFlow(command: String, argument: String) async -> Bool {
        switch command {
        case "delay":
            guard let ms = Int(argument.trimmingCharacters(in: .whitespaces)), ms >= 0 else { reply("error: expected a time in ms, got \(argument)"); return true }
            // The window keeps drawing while it waits (the display link runs on its own); the lines after this one wait.
            await pause(ms)
            reply("delay \(ms)")
        case "sync":
            // Lines run one at a time, so reaching this one means every line before it, in this batch and the ones before, is done.
            reply("sync \(argument.trimmingCharacters(in: .whitespaces))")
        case "control": reply(controlRequest(argument))
        default: return false
        }
        return true
    }

    /// `control [--from <tab>] <words…>`: the reply's one JSON line, or why the words were not a request.
    private func controlRequest(_ argument: String) -> String {
        var words = Self.splitWords(argument)
        var from: String?
        if words.first == "--from" {
            words.removeFirst()
            guard let tab = words.first else { return "error: --from needs a tab id" }
            from = tab
            words.removeFirst()
        }
        #if WITH_UNIFFI
            guard let handle = hooks?.handle else { return "error: this instance serves no control socket" }
            return handle.controlLine(words: words, from: from)
        #else
            return "error: this build has no control socket"
        #endif
    }

    /// The words as a shell would read them: split on blanks, double quotes keeping a run with blanks together.
    static func splitWords(_ text: String) -> [String] {
        var words: [String] = [], word = "", quoted = false, inWord = false
        for character in text {
            if character == "\"" {
                quoted.toggle()
                inWord = true
            } else if character.isWhitespace && !quoted {
                if inWord { words.append(word) }
                word = ""
                inWord = false
            } else {
                word.append(character)
                inWord = true
            }
        }
        if inWord { words.append(word) }
        return words
    }
}
