// What the preview is set at, which is the settings': the size (kept within what the column holds), the
// terminal's line height and whether ligatures join. Values the picker is handed, never reads.

import FontPickerModel

public struct PreviewLook: Equatable, Sendable {
    public var size: Double
    public var lineHeight: Double
    public var ligatures: Bool

    public init(size: Double, lineHeight: Double?, ligatures: Bool) {
        self.size = min(max(size, FontPickerMetric.previewSizes.lowerBound), FontPickerMetric.previewSizes.upperBound)
        self.lineHeight = lineHeight ?? FontPickerMetric.specimenLineHeight
        self.ligatures = ligatures
    }
}
