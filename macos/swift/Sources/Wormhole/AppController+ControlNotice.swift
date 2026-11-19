// The control folder's `notice <what>`: the chip of the window in front says one of the things it can say, without the event
// that causes it (a number alone is a copy of that many characters). For a check of how each looks and what it does when
// clicked; the causes themselves (a program copying, a script typing, an install finishing) are checked where they happen.

import TerminalUI

extension AppController {
    func postNotice(_ argument: String) -> String {
        let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
        let rest = words.count > 1 ? words[1] : ""
        let notices = hub.notices
        switch words.first ?? "" {
        case "chip": return "chip shown=\(notices.shown?.words ?? "none") sticky=\(notices.isClickable) info=\(notices.infoOpen) detail=\(notices.detail ?? "-") bar=\(hub.selectedBarText ?? "-") hostNotice=\(hub.selectedHostNotice ?? "-")"
        case "update-ready": notices.post(.warned(.updateReady(version: rest.isEmpty ? "9.9.9" : rest)))
        case "data-folder":
            let detail = NoticeReason.dataFolder(profile: "default", folder: dataDirectory.path, error: rest.isEmpty ? "permission denied" : rest)
            notices.post(.warned(.dataFolder), detail: detail)
        case "host-fallback": notices.post(.warned(.hostFallback), detail: NoticeReason.clean(rest.isEmpty ? "no such file: wormhole-host" : rest))
        case "sticky":
            // A warning that stays until it is clicked (`notice-click`): the session host's, whose explanation says `text`.
            notices.post(.warned(.hostFallback), detail: NoticeReason.clean(rest.isEmpty || rest == "again" ? "no such file: wormhole-host" : rest))
        case "move": notices.post(.warned(.moveToApplications))
        case "history-kept": notices.post(.failed(.historyKeptByHost))
        case "copy-path": notices.post(.failed(.copyPath))
        case "typing": notices.post(.driven(.typing(rest.isEmpty ? .scripts : .agent(rest))))
        case "reading": notices.post(.driven(.reading(rest.isEmpty ? .scripts : .agent(rest))))
        default: notices.post(.copied(.characters(Int(argument) ?? 12)))
        }
        return "notice \(argument)"
    }
}
