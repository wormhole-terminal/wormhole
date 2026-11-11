// The palette's fuzzy match from core (`view::search`): every word of a query found in a result's title (marked)
// or in its other texts, ranked best first. Core marks bytes of the title; the palette marks characters.

import PaletteModel
import WormholeBindings

struct CorePaletteMatcher: PaletteMatching {
    func rank(_ query: String, _ candidates: [PaletteCandidate]) -> [PaletteRanked] {
        let found = paletteRank(
            query: query, candidates: candidates.map { PaletteCandidateRecord(title: $0.title, other: $0.other) })
        return found.map { ranked in
            let title = candidates[Int(ranked.index)].title
            return PaletteRanked(
                index: Int(ranked.index),
                match: Match(score: Int(ranked.found.score), ranges: ranked.found.ranges.map { characters($0, in: title) }))
        }
    }
}
