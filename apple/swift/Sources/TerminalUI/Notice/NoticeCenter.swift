// One window's chip: the rules, the message on show and the timer that moves them on. A post says what and
// (for a terminal's notice) nothing about where: the host view puts the chip over the terminal. The clock is
// injected, so a test moves it; the timer only wakes the rules when a life or a leaving is over.

import Foundation
import Observation

@MainActor @Observable
public final class NoticeCenter {
    /// What the chip shows now (and while it leaves).
    public private(set) var shown: Notice?
    /// The chip is going: the view plays the way out while it is true.
    public private(set) var leaving = false
    /// A swap while showing: a view crossfades on this.
    public private(set) var serial = 0
    /// The explanation of a warning (ⓘ) was asked for; the host presents it.
    public var onInfo: ((Notice) -> Void)?
    /// A click on a warning that offers something ("Restart").
    public var onAct: ((Notice) -> Void)?
    /// The window's Reduce Motion, set by the host view: it picks the way out, which the rules need the length of.
    public var reduceMotion = false

    @ObservationIgnored private var rules = NoticeRules()
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private let now: @MainActor () -> Date

    public init(now: @escaping @MainActor () -> Date = Date.init) { self.now = now }

    public func post(_ notice: Notice) {
        let effect = rules.post(notice, now: now())
        guard effect != .ignore else { return }
        shown = notice
        leaving = false
        serial += 1
        arm()
    }

    /// The ⓘ or the capsule of a warning.
    public func clickInfo() { if let shown, shown.hasInfo { onInfo?(shown) } }

    public func clickBody() {
        guard let shown else { return }
        if rules.bodyClickActs(at: now()) { onAct?(shown) }
        else if shown.hasInfo { onInfo?(shown) }
        else { dismiss() }
    }

    public func dismiss() { apply(rules.dismiss(now: now(), leave: leave)) }
    public func withdraw() { apply(rules.withdraw(now: now(), leave: leave)) }
    public var isClickable: Bool { rules.clickable }

    private var leave: TimeInterval { NoticeTiming.leaving(reduceMotion: reduceMotion) }

    private func apply(_ effect: NoticeRules.Effect) {
        switch effect {
        case .hide: leaving = true
        case .remove: shown = nil; leaving = false
        // A copy's life is over and the warning it covered comes back.
        case .restore: shown = rules.sticky; serial += 1
        case .show, .swap, .ignore: break
        }
        arm()
    }

    /// Wakes the rules at the next due time; replaces the timer of an older one.
    private func arm() {
        timer?.cancel()
        guard let due = rules.due else { return }
        timer = Task { [weak self] in
            guard let self else { return }
            try? await Task.sleep(for: .seconds(max(due.timeIntervalSince(now()), 0)))
            guard !Task.isCancelled else { return }
            wake()
        }
    }

    /// Time moved on: a life or a leaving may be over.
    func wake() { apply(rules.advance(now: now(), leave: leave)) }
}
