// The title row's order of giving way, asked of the core (`titleFitRow`, core's `view::layout::title_row::fit_row`): Swift
// measures the row's texts in its own fonts, hands the widths over and draws what comes back.

import PanesModel
import WormholeBindings

public struct CoreRowFitting: RowFitting {
    public init() {}

    public func fit(avail: Double, _ wants: RowWants) -> RowFit {
        let answer = titleFitRow(avail: Float(avail), wants: TitleRowWants(wants))
        return RowFit(answer)
    }
}

extension TitleRowWants {
    init(_ wants: RowWants) {
        func width(_ extra: Extra) -> Float? { wants.extras.width(extra).map(Float.init) }
        self.init(
            path: wants.path.map(Float.init), words: Float(wants.words), chipNatural: wants.chip.map { Float($0.natural) },
            chipIcon: wants.chip.map { Float($0.icon) }, extras: Extra.allCases.map(width),
            portsNarrower: wants.portsNarrower.map(Float.init), group: Float(wants.group), folded: Float(wants.folded),
            name: Float(wants.name), nameMin: Float(wants.nameMin), fixed: Float(wants.fixed))
    }
}

extension RowFit {
    init(_ core: TitleRowFit) {
        var extras = Extras()
        for (extra, width) in zip(Extra.allCases, core.extras) { extras = extras.with(extra, width.map(Double.init)) }
        let path: PathShown =
            switch core.path {
            case .none: .none
            case .icon: .icon
            case .step(let index): .step(Int(index))
            }
        self.init(
            path: path, words: core.words, chip: core.chip.map(Double.init), extras: extras, portsStep: Int(core.portsStep),
            folded: core.folded, name: Double(core.name))
    }
}
