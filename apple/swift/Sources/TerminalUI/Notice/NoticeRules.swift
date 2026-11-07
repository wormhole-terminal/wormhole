// When a notice is shown, swapped, dropped or taken down (`view::notice::machine::ChipRules`), without a clock
// of its own: every call says what time it is, so the rules are tested by moving the time.

import Foundation

struct NoticeRules {
    enum Phase: Equatable { case hidden, showing, leaving }
    enum Effect: Equatable { case show, swap, restore, hide, remove, ignore }

    private enum State: Equatable {
        case hidden
        /// `until` nil: it stays (a warning).
        case showing(until: Date?)
        case leaving(until: Date)
    }

    private var state = State.hidden
    private var last: (Notice, Date)?
    private(set) var sticky: Notice?
    private var dismissed: Set<Notice> = []
    private var armedAt: Date?

    /// A notice arrives. A copy is shown for `life`; a warning stays until clicked, and is not said again once closed.
    mutating func post(_ notice: Notice, now: Date) -> Effect {
        if notice.isSticky {
            guard sticky != notice, !dismissed.contains(notice) else { return .ignore }
            sticky = notice
            armedAt = notice.hasAction ? now.addingTimeInterval(NoticeTiming.arm) : nil
            return show(until: nil)
        }
        if let (before, at) = last, before == notice, now.timeIntervalSince(at) < NoticeTiming.dedupe { return .ignore }
        last = (notice, now)
        return show(until: now.addingTimeInterval(NoticeTiming.life))
    }

    private mutating func show(until: Date?) -> Effect {
        let effect: Effect = if case .showing = state { .swap } else { .show }
        state = .showing(until: until)
        return effect
    }

    /// Time moved on: a copy's life may be over (a warning behind it comes back), a leaving may be done.
    mutating func advance(now: Date, leave: TimeInterval) -> Effect {
        switch state {
        case let .showing(until?) where now >= until:
            if sticky != nil { state = .showing(until: nil); return .restore }
            state = .leaving(until: now.addingTimeInterval(leave))
            return .hide
        case let .leaving(until) where now >= until:
            state = .hidden
            return .remove
        default: return .ignore
        }
    }

    /// The person clicked the warning's ×: it goes for good.
    mutating func dismiss(now: Date, leave: TimeInterval) -> Effect {
        guard clickable else { return .ignore }
        if let sticky { dismissed.insert(sticky) }
        sticky = nil
        state = .leaving(until: now.addingTimeInterval(leave))
        return .hide
    }

    /// Whatever the warning was about is gone (the update was installed): it leaves without being dismissed.
    mutating func withdraw(now: Date, leave: TimeInterval) -> Effect {
        guard sticky != nil else { return .ignore }
        sticky = nil
        guard case .showing = state else { return .ignore }
        state = .leaving(until: now.addingTimeInterval(leave))
        return .hide
    }

    /// When `advance` next has something to do.
    var due: Date? {
        switch state {
        case .hidden, .showing(until: nil): nil
        case let .showing(until?), let .leaving(until): until
        }
    }

    var phase: Phase {
        switch state {
        case .hidden: .hidden
        case .showing: .showing
        case .leaving: .leaving
        }
    }

    /// Only a warning that stays takes clicks.
    var clickable: Bool { state == .showing(until: nil) }

    /// A click on the capsule itself, at `time`: held back while the offer has only just appeared.
    func bodyClickActs(at time: Date) -> Bool {
        guard let sticky, sticky.hasAction, clickable else { return false }
        return armedAt.map { time >= $0 } ?? true
    }
}
