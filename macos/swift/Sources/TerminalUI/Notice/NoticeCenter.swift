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
    /// What the error behind the warning on show said, one short line (core's `reason`): the explanation under ⓘ quotes it.
    public private(set) var detail: String?
    /// The explanation under the ⓘ is open.
    public var infoOpen = false
    /// Whether the Notices feature is on in this window's profile. Off, nothing drops onto the terminal: what is worth saying
    /// goes to `onBar` (the pane's bar, `terminal_view/pane_bar.rs`).
    public var chipOn: () -> Bool = { true }
    public var onBar: ((Notice, String?) -> Void)?
    /// Open Logs Folder, in the explanation.
    public var onOpenLogs: (() -> Void)?
    /// A click on a warning that offers something ("Restart").
    public var onAct: ((Notice) -> Void)?
    /// The × (or a click that only dismisses) closed the notice on show.
    public var onDismissed: ((Notice) -> Void)?
    /// The window's Reduce Motion, set by the host view: it picks the way out, which the rules need the length of.
    public var reduceMotion = false

    @ObservationIgnored private var rules = NoticeRules()
    @ObservationIgnored private var timer: Task<Void, Never>?
    @ObservationIgnored private let now: @MainActor () -> Date

    public init(now: @escaping @MainActor () -> Date = Date.init) { self.now = now }

    /// `detail` is what went wrong, for a warning that explains itself.
    public func post(_ notice: Notice, detail: String? = nil) {
        guard chipOn() else { return onBar?(notice, detail) ?? () }
        let effect = rules.post(notice, now: now())
        guard effect != .ignore else { return }
        if notice.isSticky { self.detail = detail }
        shown = notice
        leaving = false
        serial += 1
        arm()
    }

    /// The ⓘ or the capsule of a warning.
    public func clickInfo() { if shown?.hasExplanation == true { infoOpen.toggle() } }

    public func clickBody() {
        guard let shown else { return }
        if rules.bodyClickActs(at: now()) { onAct?(shown) }
        else if shown.hasInfo { clickInfo() }
        else { dismiss() }
    }

    public func dismiss() {
        if let shown { onDismissed?(shown) }
        apply(rules.dismiss(now: now(), leave: leave))
    }
    public func withdraw() { apply(rules.withdraw(now: now(), leave: leave)) }
    public var isClickable: Bool { rules.clickable }

    private var leave: TimeInterval { NoticeTiming.leaving(reduceMotion: reduceMotion) }

    private func apply(_ effect: NoticeRules.Effect) {
        switch effect {
        case .hide: leaving = true; infoOpen = false
        case .remove: shown = nil; leaving = false; infoOpen = false
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
