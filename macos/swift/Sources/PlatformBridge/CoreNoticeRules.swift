// The notification rules from core (`notify`), behind `NoticeDeciding`: a look at the terminals becomes the
// banners, retractions, bounce and spoken words the rules decide on. One `Notifier` holds what they remember
// (what was seen last, the recent burst, the throttles).

import PlatformModel
import WormholeBindings

final class CoreNoticeRules: NoticeDeciding {
    private let rules = Notifier()

    func observe(
        _ tabs: [NoticeTab], settings: NoticeSettings, appActive: Bool, timeSensitive: Bool, nowMs: UInt64,
        wallMs: UInt64
    ) -> [NoticeAction] {
        rules.observe(
            tabs: tabs.map(\.record), settings: settings.record, appActive: appActive, timeSensitive: timeSensitive,
            nowMs: nowMs, wallMs: wallMs
        ).map(NoticeAction.init)
    }

    func dismiss(tab: Int64) -> String? { rules.dismiss(tab: tab) }

    func respond(_ response: NoticeResponse) -> (outcome: NoticeOutcome, retract: String?) {
        let answer = rules.respond(response: ResponseRecord(id: response.id, action: response.action, reply: response.reply))
        return (NoticeOutcome(answer.kind), answer.retract)
    }

    func testNotice(name: String, settings: NoticeSettings, timeSensitive: Bool) -> Notice {
        Notice(rules.testNotice(name: name, settings: settings.record, timeSensitive: timeSensitive))
    }
}
