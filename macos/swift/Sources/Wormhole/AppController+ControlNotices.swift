// The control folder's notice and notification commands that GPUI's devctl has (`devctl/notices.rs`, `ambient.rs`):
// `notice-click [close|info|body]`, `notify-sim <kind> <n> [post]` and `notify-permission <state>`; `notice sticky [text]` is a
// word of `notice` (`AppController+ControlNotice`). A test instance never shows a banner or asks the system: the banners go to
// its log (`notice-log`), the permission is the one the script says. `notice-material` has no counterpart: the Swift chip is
// always Liquid Glass.

import AppKit
import Foundation
import PlatformModel
#if WITH_UNIFFI
    import PlatformBridge
#endif

extension AppController {
    /// False when `command` is not one of these.
    func runNotices(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "notice-click":
            // The × (the default) dismisses a sticky notice; the ⓘ and the capsule open its explanation, or dismiss it where it has none.
            let chip = hub.notices
            guard chip.shown != nil else { reply("error: no chip to click in this window"); return true }
            switch argument {
            case "", "close": if chip.isClickable { chip.dismiss() }
            case "info": chip.clickInfo()
            case "body": chip.clickBody()
            default: reply("error: expected close, info or body, got \(argument)"); return true
            }
            reply("notice-click \(argument.isEmpty ? "close" : argument)")
        case "notify-sim": reply(notifySim(argument))
        case "notify-permission":
            #if WITH_UNIFFI
                guard let notifier = notices?.notifier as? LogNotifier else { reply("error: this instance has a real notification centre"); return true }
                let states: [String: NoticePermission] = [
                    "not-asked": .notAsked, "allowed": .allowed(timeSensitive: false), "denied": .denied, "outside": .unavailable, "checking": .checking,
                ]
                guard let permission = states[argument] else { reply("error: permission \(argument): not-asked, allowed, denied, outside or checking"); return true }
                notifier.pretend(permission)
                hooks?.tellPermission()
                reply("permission=\(permission)")
            #else
                reply("error: this build has no core")
            #endif
        default: return false
        }
        return true
    }

    /// `notify-sim <kind> <n> [post]`: what the rules decide for that event in the n-th terminal (delivery, level, sound, bounce,
    /// the words); with `post`, delivered through the notifier this instance has (its log).
    private func notifySim(_ argument: String) -> String {
        #if WITH_UNIFFI
            let words = argument.split(separator: " ").map(String.init)
            let kinds: [String: NoticeKind] = [
                "needs-you": .needsYou, "agent-done": .agentDone, "command-done": .commandDone, "command-failed": .commandFailed,
                "agent-failed": .agentFailed, "bell": .bell, "notice": .notice,
            ]
            guard let kind = words.first.flatMap({ kinds[$0] }) else { return "error: kind \(words.first ?? ""): needs-you, agent-done, agent-failed, command-done, command-failed, bell or notice" }
            let terminals = ambientTerminals
            let n = words.count > 1 ? Int(words[1]) ?? 0 : 1
            guard terminals.indices.contains(n - 1) else { return "error: no terminal \(words.count > 1 ? words[1] : "1") in this window" }
            let terminal = terminals[n - 1]
            let holder = session(holding: Int(terminal.session))
            let active = NSApp.isActive
            let looking = active && holder?.window?.isKeyWindow == true && holder?.hub.state.selected == Int(terminal.session)
            let said = PlatformBridge.simulate(
                kind, settings: settingsNotice, looking: looking, appActive: active, timeSensitive: false, name: terminal.name, who: terminal.who,
                folder: terminal.folder, branch: terminal.branch, message: terminal.message)
            if words.last == "post", said.banner, let notifier = notices?.notifier {
                notifier.post(Notice(id: "wormhole/sim/\(terminal.tab)", thread: "wormhole/sim/\(terminal.tab)", title: said.title, subtitle: said.subtitle, body: said.body, level: said.level, sound: said.sound))
            }
            return "notify-sim \(kind) tab=\(terminal.tab) app_active=\(active) looking=\(looking) -> \(said.banner ? "Banner" : "Nothing") level=\(said.level) "
                + "sound=\(said.sound) bounce=\(said.bounce) title=\(said.title.debugDescription) subtitle=\(said.subtitle.debugDescription) body=\(said.body.debugDescription)"
        #else
            return "error: this build has no core"
        #endif
    }
}
