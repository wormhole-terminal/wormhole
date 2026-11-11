// The notification rules' records as `PlatformModel`'s values, and back.

import PlatformModel
import WormholeBindings

extension NoticeSettings {
    var record: NotifySettingsRecord {
        NotifySettingsRecord(
            notifications: notifications, needsYou: needsYou, agentDone: agentDone, commandDone: commandDone,
            commandFailed: commandFailed, commandAfterSecs: commandAfterSecs, bell: bell, onlyUnseen: onlyUnseen,
            bannerInFront: bannerInFront, sound: sound.record, groupBursts: groupBursts, dockBounce: dockBounce)
    }
}

extension NoticeSettings {
    /// What core starts a person's settings from, so Swift writes no default of its own.
    init(_ record: NotifySettingsRecord) {
        self.init()
        notifications = record.notifications
        needsYou = record.needsYou
        agentDone = record.agentDone
        commandDone = record.commandDone
        commandFailed = record.commandFailed
        commandAfterSecs = record.commandAfterSecs
        bell = record.bell
        onlyUnseen = record.onlyUnseen
        bannerInFront = record.bannerInFront
        sound =
            switch record.sound {
            case .needsYou: .needsYou
            case .all: .all
            case .none: .none
            }
        groupBursts = record.groupBursts
        dockBounce = record.dockBounce
    }
}

extension NoticeSettings.Sound {
    var record: SoundChoiceRecord {
        switch self {
        case .needsYou: .needsYou
        case .all: .all
        case .none: .none
        }
    }
}

extension NoticeState {
    var record: NoticeStateRecord {
        switch self {
        case .idle: .idle
        case .working: .working
        case .needsYou: .needsYou
        case .done: .done
        case .failed: .failed
        case .exited: .exited
        }
    }
}

extension NoticeSource {
    var record: NoticeSourceRecord {
        switch self {
        case .plain: .plain
        case .agent: .agent
        case .program: .program
        }
    }
}

extension NoticeTab {
    var record: TabFactsRecord {
        TabFactsRecord(
            session: session, tab: tab, name: name, state: state.record, sinceMs: sinceMs, who: who,
            source: source.record, message: message, reported: reported,
            failure: failure.map {
                FailureRecord(
                    serial: $0.serial, code: $0.code, atMs: $0.atMs, command: $0.command, tookMs: $0.tookMs,
                    lastLine: $0.lastLine)
            },
            notice: notice.map { ProgramNoticeRecord(serial: $0.serial, title: $0.title, body: $0.body) },
            foreground: foreground.map { ForegroundRecord(command: $0.command, startedUnixMs: $0.startedUnixMs) },
            folder: folder, branch: branch, changed: changed, selected: selected, key: key, settings: nil)
    }
}

extension Notice {
    init(_ record: NoticeRecord) {
        let level: NoticeLevel =
            switch record.level {
            case .passive: .passive
            case .active: .active
            case .timeSensitive: .timeSensitive
            }
        let category: NoticeCategory =
            switch record.category {
            case .needsYou: .needsYou
            case .finished: .finished
            case .plain: .plain
            }
        self.init(
            id: record.id, thread: record.thread, title: record.title, subtitle: record.subtitle, body: record.body,
            level: level, sound: record.sound, category: category)
    }
}

extension NoticeAction {
    init(_ record: NotifyActionRecord) {
        self =
            switch record {
            case .post(let notice): .post(Notice(notice))
            case .retract(let id): .retract(id: id)
            case .bounce: .bounce
            case .say(let text, let urgent): .say(text, urgent: urgent)
            }
    }
}

extension NoticeOutcome {
    init(_ record: ResponseKindRecord) {
        self =
            switch record {
            case .nothing: .nothing
            case .activate: .activate
            case .open(let tab): .open(tab: tab)
            case .showChanges(let tab): .showChanges(tab: tab)
            case .reply(let tab, let text): .reply(tab: tab, text: text)
            }
    }
}
