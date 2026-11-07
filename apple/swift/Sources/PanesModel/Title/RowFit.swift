// The row's order of giving way in `avail` points, each step only when the one before did not make it fit:
//
// 1. the path loses its middle, down to `~/…/last folder`;
// 2. the status words go, whole;
// 3. the extras go, one at a time and whole, in `Extra.giveWay` order (the ports first lose their chips);
// 4. the branch chip shrinks to its glyph;
// 5. Open in… and the splits fold into one `⋯` menu;
// 6. the name is cut at its end, to a few letters;
// 7. the path folds to its folder's icon, and the name takes what that frees.

import Foundation

public func fitRow(avail: Double, _ wants: RowWants) -> RowFit {
    var path: PathShown = wants.path.isEmpty ? .none : .step(0)
    var words = wants.words > 0
    var extras = wants.extras
    var chip = wants.chip?.natural
    var folded = false

    // What the row takes besides the name.
    func rest() -> Double {
        var width = wants.fixed + (words ? wants.words : 0) + extras.total + (chip ?? 0)
        if case .step(let ix) = path { width += wants.path[ix] }
        return width + (folded ? wants.folded : wants.group)
    }
    func fits() -> Bool { rest() + wants.name <= avail }

    while case .step(let ix) = path, ix + 1 < wants.path.count, !fits() { path = .step(ix + 1) }
    if !fits() { words = false }
    var portsStep = 0
    for extra in Extra.giveWay {
        // The ports give up a chip at a time before they go.
        while extra == .ports, extras.shows(extra), portsStep < wants.portsNarrower.count, !fits() {
            extras = extras.with(extra, wants.portsNarrower[portsStep])
            portsStep += 1
        }
        if !fits() { extras = extras.with(extra, nil) }
    }
    if let width = chip, let shape = wants.chip, !fits() {
        let over = rest() + wants.name - avail
        chip = max(width - over, min(shape.icon, width))
    }
    if !fits() { folded = true }

    let floor = min(wants.nameMin, wants.name)
    var name = min(wants.name, avail - rest())
    if name < floor, case .step = path {
        path = .icon
        name = min(wants.name, avail - rest())
    }
    // Whatever is left of the row, which clips the name if even that is nothing.
    return RowFit(path: path, words: words, chip: chip, extras: extras, portsStep: portsStep, folded: folded, name: max(name, 0))
}
