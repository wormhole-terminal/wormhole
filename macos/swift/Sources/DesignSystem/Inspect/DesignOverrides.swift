// The inspector's nudges (debug builds only; a release build has none of this file): one table the scales consult
// (`ScaleValue.of`, which `Space`, `Metric` and `Radius` read through), keyed as the inspector names a value: `space._1`,
// `metric.keyCapHeight`, `radius.chip`. A nudge writes it and every view that read a scale value in its body draws again.
// Kept for the session only: no file, no defaults.

#if DEBUG
    import Foundation
    import Observation
    import Synchronization

    /// The scale values nudged in this session, and the standard of every value read so far.
    public final class DesignOverrides: Sendable, Observable {
        /// The one table of the app.
        public static let shared = DesignOverrides()

        private struct Table {
            var nudged: [String: CGFloat] = [:]
            var standards: [String: CGFloat] = [:]
        }
        private let table = Mutex(Table())
        private let registrar = ObservationRegistrar()
        /// What a view that read a value is told changed (the table itself is behind the lock).
        var revision: Int { 0 }

        func value(_ key: String, _ standard: CGFloat) -> CGFloat {
            registrar.access(self, keyPath: \.revision)
            return table.withLock { table in
                table.standards[key] = standard
                return table.nudged[key] ?? standard
            }
        }

        /// The value's standard and what it is nudged to, once something read it; nil for a key no scale has.
        public func state(of key: String) -> (standard: CGFloat, nudged: CGFloat?)? {
            registrar.access(self, keyPath: \.revision)
            return table.withLock { table in table.standards[key].map { ($0, table.nudged[key]) } }
        }

        /// Nudges `key` to `value`; nil, or its standard, takes the nudge away.
        public func set(_ key: String, to value: CGFloat?) {
            registrar.withMutation(of: self, keyPath: \.revision) {
                table.withLock { table in table.nudged[key] = value == table.standards[key] ? nil : value }
            }
        }

        /// Every value back to its standard.
        public func reset() {
            registrar.withMutation(of: self, keyPath: \.revision) { table.withLock { $0.nudged = [:] } }
        }
    }
#endif
