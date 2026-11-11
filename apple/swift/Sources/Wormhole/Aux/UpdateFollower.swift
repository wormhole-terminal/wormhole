// The updater between launches (`update/mod.rs`, `update/prompt.rs`, `update/restart.rs`): the schedule's checks, which download and
// install by themselves when Settings says so; the chip that says an update is ready, whose click restarts (asking first when a
// restart would end something); and "Restart When Idle", which waits until every terminal has been idle for a while. The check,
// the download, the signature and the swap are the core's.

import AppKit
import AuxModel
import AuxWindows
import Observation
import TerminalUI
import WorkspaceModel

#if WITH_UNIFFI
    import AuxBridge
#endif

@MainActor
final class UpdateFollower {
    private unowned let app: AppController
    private let backend: any UpdateBackend
    private var following: Task<Void, Never>?
    /// The version whose chip is up, so one install says it once.
    private var told: String?
    /// "Restart When Idle" was chosen: what restarts, once the terminals have been idle long enough.
    private var pending: (@MainActor () -> Void)?
    private var watch = IdleWatch()
    private var looks = 0

    init(app: AppController, backend: any UpdateBackend) { (self.app, self.backend) = (app, backend) }

    func start() {
        following = Task { [weak self, backend] in
            for await phase in backend.phases() { self?.phase(phase) }
        }
    }

    private func phase(_ phase: UpdatePhase) {
        switch phase {
        // The core installs what an automatic check finds, as auto_update says (and not again a version that failed).
        case .installed(let version) where told != version:
            told = version
            app.hub.notices.post(.warned(.updateReady(version: version)))
        case .installed: break
        default:
            // Whatever the chip was about is gone (a channel change called the install off).
            if told != nil { told = nil; app.sessions.forEach { $0.hub.notices.withdraw() } }
            pending = nil
        }
    }

    /// A click on the chip, or Restart in Settings: at once when nothing would end, else the question first.
    func requestRestart() {
        guard let question = RestartQuestion.question(app.runningTerminals) else { return backend.restart(whenIdle: false) }
        Task {
            switch await QuestionSheet(question).ask(in: app.window) {
            case .confirm: backend.restart(whenIdle: false)
            case .also: backend.restart(whenIdle: true)
            case .cancel: break
            }
        }
    }

    /// Restart once every terminal has been idle for `RestartIdle.idleFor`: looked at when one changes and when the wait is due,
    /// never on a clock that ticks.
    func restartWhenIdle(_ restart: @escaping @MainActor () -> Void) {
        (pending, watch) = (restart, IdleWatch())
        look()
    }

    private func look() {
        guard let restart = pending else { return }
        looks += 1
        let generation = looks
        let idle = withObservationTracking { app.allTerminalsIdle } onChange: { [weak self] in
            Task { @MainActor in if self?.looks == generation { self?.look() } }
        }
        if watch.seen(idle: idle, at: Date()) {
            pending = nil
            return restart()
        }
        guard idle else { return }
        Task { [weak self] in
            // one-shot: the moment every terminal has been idle for idleFor, armed when one went idle
            try? await Task.sleep(for: .seconds(RestartIdle.idleFor))
            if self?.looks == generation { self?.look() }
        }
    }
}

extension AppController {
    func startUpdates() {
        guard options.updateFeedURL != nil else { return }
        let follower = UpdateFollower(app: self, backend: aux.updateBackend())
        updateFollower = follower
        follower.start()
    }

    /// Every terminal lets "Restart When Idle" go ahead (`update::restart::is_idle`).
    var allTerminalsIdle: Bool {
        for session in sessions {
            for terminal in session.hub.state.all where !terminal.isIdleForRestart { return false }
        }
        return true
    }
}

private extension TerminalSummary {
    var isIdleForRestart: Bool {
        RestartIdle.isIdle(runStatus, ended: activity == .exited, commandRunning: runningCommand != nil, agentInFront: agent != nil)
    }
}
