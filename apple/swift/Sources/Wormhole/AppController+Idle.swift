// The idle check, for `--idle <seconds>`.

import AppKit
import Foundation
import TerminalMetal

extension AppController {
    /// Hold the window idle, then print what the kernel counted: process CPU time and wake-ups. A one-shot
    /// timer at each end of the interval is the only thing this adds to the process.
    func scheduleIdleReport(after seconds: Double) {
        guard let view = selectedView else { return }
        // Let the first frames and any start-up work finish, then stop the source: the window has nothing to show.
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            MainActor.assumeIsolated {
                view.source.stop()
                let cpu0 = WakeupCounter.processCPUNanoseconds()
                let wake0 = WakeupCounter.current()
                let stats0 = view.statistics
                DispatchQueue.main.asyncAfter(deadline: .now() + seconds) {
                    MainActor.assumeIsolated {
                        let cpu = WakeupCounter.processCPUNanoseconds() - cpu0
                        let stats = view.statistics
                        var report = String(
                            format: "idle %.1fs: cpu=%.2fms ticks=%d frames=%d wakes=%d blink=%d link=%@",
                            seconds, Double(cpu) / 1e6, stats.ticks - stats0.ticks, stats.framesDrawn - stats0.framesDrawn,
                            stats.wakes - stats0.wakes, stats.blinkFires - stats0.blinkFires, view.isLinkRunning ? "running" : "paused")
                        if let wake0, let wake1 = WakeupCounter.current() {
                            report += " interrupt_wakeups=\(wake1.interrupts - wake0.interrupts)"
                                + " platform_idle_wakeups=\(wake1.platformIdle - wake0.platformIdle)"
                        }
                        print(report)
                        self.devctl?.append(report)
                        self.terminateUnattended()
                    }
                }
            }
        }
    }
}
