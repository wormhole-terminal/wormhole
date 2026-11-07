// The sentence of a notice (`apple` side of `app/notice/words.rs`): what VoiceOver says is the same words.

extension Notice {
    public var words: String {
        switch self {
        case let .copied(copied):
            switch copied {
            case let .lines(n): L10n.noticeCopiedLines(n)
            case let .characters(n): L10n.noticeCopiedCharacters(n)
            case let .programLines(n): L10n.noticeProgramLines(n)
            case let .programCharacters(n): L10n.noticeProgramCharacters(n)
            case let .output(lines): L10n.noticeCopiedOutput(lines)
            case .command: L10n.noticeCopiedCommand
            case .path: L10n.noticeCopiedPath
            case .error: L10n.noticeCopiedError
            }
        case let .driven(driven):
            switch driven {
            case let .typing(.agent(name)): L10n.noticeTyping(name)
            case let .reading(.agent(name)): L10n.noticeReading(name)
            case .typing(.scripts): L10n.noticeScriptTyping
            case .reading(.scripts): L10n.noticeScriptReading
            }
        case let .failed(failed):
            switch failed {
            case .copyPath: L10n.noticeFailedCopyPath
            case .historyKeptByHost: L10n.noticeHistoryKept
            case let .putAtPrompt(refusal):
                switch refusal {
                case .concealed: L10n.noticePutConcealed
                case .sameColor: L10n.noticePutSameColor
                case .invisible: L10n.noticePutInvisible
                case .lines: L10n.noticePutLines
                }
            }
        case let .warned(warning):
            switch warning {
            case .hostFallback: L10n.noticeHostFallback
            case .dataFolder: L10n.noticeDataFolder
            case .moveToApplications: L10n.noticeMoveToApplications
            case let .updateReady(version): L10n.noticeUpdateReady(version)
            }
        }
    }
}
