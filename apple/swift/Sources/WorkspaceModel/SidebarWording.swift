// The words of a row: its second line and how its times read, after `wormhole-core`
// `view/sidebar/wording.rs` and the app's `sidebar/row/second_line.rs`. Pure: the clock comes in as `now`,
// so the same facts and instant always give the same words.

import Foundation

/// What colour a second line takes, by meaning; the design system maps each to a token.
public enum SecondLineTone: Sendable, Equatable {
    case plain, attention, done, failed
}

/// A row's second line: its words, their tone, whether they are a command (code font; a path is in the
/// interface font) and what sits at the line's right.
public struct SecondLine: Sendable, Equatable {
    public var text: String
    public var tone: SecondLineTone = .plain
    public var isCommand = false
    public var trail: String?
}

public enum SidebarWording {
    /// How many characters of a path fit on the second line before it is cut from the front.
    public static let placeChars = 38

    /// The second line of every regular row: what the terminal has to report comes first (an agent's state
    /// in words; a plain terminal's command, or its last failure); a quiet one says where it is, and where
    /// even that is unknown what it is, so no row is a single line. `home` shortens a path under it to `~`.
    public static func secondLine(for terminal: TerminalSummary, now: Date, home: String? = nil) -> SecondLine {
        let place = terminal.folder.map { pathTail(displayPath($0, home: home), max: placeChars) }
        if terminal.agent != nil {
            let since = elapsed(terminal.activitySince, now)
            if let text = agentDetail(terminal.activity, message: terminal.message, since: since) {
                return SecondLine(text: text, tone: tone(of: terminal.activity))
            }
            return SecondLine(text: place ?? "Idle")
        }
        let trail = terminal.unread > 0 ? "new output" : terminal.runningSince.map { clock(elapsed($0, now)) }
        if let command = terminal.runningCommand {
            return SecondLine(text: command, isCommand: true, trail: trail)
        }
        // The last command failed: what it was and how it ended, in danger, and when, until the next starts.
        if let failure = terminal.lastFailure {
            let exit = "exit \(failure.exitCode)"
            return SecondLine(
                text: failure.command.map { "\($0) · \(exit)" } ?? exit, tone: .failed, isCommand: true,
                trail: ago(elapsed(failure.at, now)))
        }
        return SecondLine(text: place ?? "Shell")
    }

    /// How long until the second line's words next read differently: the next minute of an agent's status or
    /// a failure, the next second of a command's clock. For arming one timer, never polling.
    public static func nextChange(for terminal: TerminalSummary, now: Date) -> TimeInterval? {
        let minute: TimeInterval = 60
        if terminal.agent != nil {
            guard terminal.message == nil, terminal.activity != .idle, terminal.activity != .needsInput,
                let since = terminal.activitySince
            else { return nil }
            return untilNext(elapsed(since, now), unit: minute)
        }
        if let since = terminal.runningSince, terminal.runningCommand != nil {
            return untilNext(elapsed(since, now), unit: 1)
        }
        return terminal.lastFailure.map { untilNext(elapsed($0.at, now), unit: minute) }
    }

    /// "Needs input · Approve cargo test?", "Editing sidebar.rs…" (its own words), "Working · 3m" ("<1m" in
    /// the first minute), "Finished · 12m ago", "Failed · rate limit reached"; nil for an idle agent.
    static func agentDetail(_ activity: Activity, message: String?, since: TimeInterval) -> String? {
        let message = message?.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty
        switch activity {
        case .needsInput: return message.map { "Needs input · \($0)" } ?? "Needs input"
        case .working: return message ?? "Working · \(took(since) ?? "<1m")"
        case .finished: return "Finished · \(message ?? ago(since))"
        case .failed: return "Failed · \(message ?? ago(since))"
        case .idle: return nil
        }
    }

    /// How long something has taken, short: nil under a minute, then "3m", "1h", "2d".
    static func took(_ elapsed: TimeInterval) -> String? {
        let minutes = Int(elapsed) / 60
        switch minutes {
        case 0: return nil
        case 1..<60: return "\(minutes)m"
        case 60..<1440: return "\(minutes / 60)h"
        default: return "\(minutes / 1440)d"
        }
    }

    /// How long ago, short: "just now", "12m ago", "1h ago", "2d ago".
    static func ago(_ elapsed: TimeInterval) -> String {
        took(elapsed).map { "\($0) ago" } ?? "just now"
    }

    /// A running command's time on the clock: "0:05", "1:12", "1:02:03".
    static func clock(_ elapsed: TimeInterval) -> String {
        let secs = Int(elapsed)
        let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60)
        return hours > 0
            ? String(format: "%d:%02d:%02d", hours, minutes, seconds) : String(format: "%d:%02d", minutes, seconds)
    }

    /// `~/Projects/app` for a path under `home`, the path itself otherwise.
    static func displayPath(_ path: String, home: String?) -> String {
        guard let home, !home.isEmpty else { return path }
        if path == home { return "~" }
        return path.hasPrefix(home + "/") ? "~" + path.dropFirst(home.count) : path
    }

    /// A path for a narrow line: cut from the front, so the end, which tells two apart, stays
    /// (`…/clients/acme/app`). Keeps whole components; `max` counts characters.
    static func pathTail(_ path: String, max: Int) -> String {
        guard path.count > max else { return path }
        var tail = ""
        for part in path.split(separator: "/").reversed() {
            let next = tail.isEmpty ? String(part) : "\(part)/\(tail)"
            // "…/" takes two characters.
            if next.count + 2 > max && !tail.isEmpty { break }
            tail = next
        }
        return "…/\(tail)"
    }

    private static func untilNext(_ elapsed: TimeInterval, unit: TimeInterval) -> TimeInterval {
        unit - elapsed.truncatingRemainder(dividingBy: unit)
    }

    private static func elapsed(_ since: Date?, _ now: Date) -> TimeInterval {
        since.map { Swift.max(0, now.timeIntervalSince($0)) } ?? 0
    }

    private static func tone(of activity: Activity) -> SecondLineTone {
        switch activity {
        case .needsInput: .attention
        case .finished: .done
        case .failed: .failed
        case .working, .idle: .plain
        }
    }
}

extension String {
    fileprivate var nilIfEmpty: String? { isEmpty ? nil : self }
}
