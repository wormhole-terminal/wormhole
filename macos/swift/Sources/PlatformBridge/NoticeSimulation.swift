// What the notification rules decide for one kind of event, without the event (core's `delivery`, `interruption_level`,
// `sound`, `bounces` and `text`, through `notifySimulate`): the control folder's `notify-sim`. Nothing is posted here.

import PlatformModel
import WormholeBindings

public struct NoticeSimulation: Sendable, Equatable {
    public var banner: Bool
    public var level: NoticeLevel
    public var sound: Bool
    public var bounce: Bool
    public var title: String
    public var subtitle: String
    public var body: String
}

extension PlatformBridge {
    /// The rules' answer for `kind` in a terminal called `name`, under `settings`.
    public static func simulate(
        _ kind: NoticeKind, settings: NoticeSettings, looking: Bool, appActive: Bool, timeSensitive: Bool, name: String, who: String?,
        folder: String?, branch: String?, message: String?
    ) -> NoticeSimulation {
        let record: NoticeKindRecord =
            switch kind {
            case .needsYou: .needsYou
            case .agentDone: .agentDone
            case .commandDone: .commandDone
            case .commandFailed: .commandFailed
            case .agentFailed: .agentFailed
            case .bell: .bell
            case .notice: .notice
            }
        let said = notifySimulate(
            kind: record, settings: settings.record, looking: looking, appActive: appActive, timeSensitive: timeSensitive, name: name, who: who,
            folder: folder, branch: branch, message: message)
        let level: NoticeLevel =
            switch said.level {
            case .passive: .passive
            case .active: .active
            case .timeSensitive: .timeSensitive
            }
        return NoticeSimulation(
            banner: said.banner, level: level, sound: said.sound, bounce: said.bounce, title: said.title, subtitle: said.subtitle, body: said.body)
    }
}
