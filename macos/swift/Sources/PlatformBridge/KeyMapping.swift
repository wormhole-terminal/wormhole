// Core's key records as `PlatformModel`'s values, and back.

import PlatformModel
import WormholeBindings

extension KeySpec {
    init(_ record: KeySpecRecord) { self.init(KeyMods(rawValue: Int(record.mods)), record.key) }

    var record: KeySpecRecord { KeySpecRecord(mods: UInt8(truncatingIfNeeded: mods.rawValue), key: key) }
}

extension KeyScope {
    init(_ record: KeyScopeRecord) {
        self =
            switch record {
            case .app: .app
            case .edit: .edit
            case .terminal: .terminal
            case .list: .list
            case .studio: .studio
            }
    }

    var record: KeyScopeRecord {
        switch self {
        case .app: .app
        case .edit: .edit
        case .terminal: .terminal
        case .list: .list
        case .studio: .studio
        }
    }
}

extension KeymapEntry {
    init(_ record: KeymapEntryRecord) {
        let stage: KeyStage =
            switch record.stage {
            case .now: .now
            case .later: .later
            case .system: .system
            case .reserved: .reserved
            }
        self.init(
            id: ActionID(rawValue: record.id), title: record.title, titleOff: record.titleOff, name: record.name,
            keys: record.keys.map(KeySpec.init), scope: KeyScope(record.scope), stage: stage,
            palette: record.palette, debugOnly: record.debugOnly, persistentOnly: record.persistentOnly,
            feature: record.feature, flag: record.flag)
    }
}
