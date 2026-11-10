// The tokens of the specimens that are the system's controls, or numbers (shape, inputs, menus, controls): what they draw with, by hand.
// The other specimens' lists are generated from the sources (`GalleryUses`).

enum SpecimenTokens {
    static let shape = [
        "metrics.radius_sm", "metrics.radius_chip", "metrics.radius", "metrics.radius_lg", "metrics.border_width",
        "metrics.divider_width", "metrics.focus_ring_width", "border", "border.strong", "border.focus", "divider",
        "chip.border", "chip.tint",
    ]
    static let inputs = [
        "primary", "input.background", "input.border", "input.border.focus", "input.placeholder", "checkbox.background",
        "checkbox.checked", "checkbox.border", "switch.knob",
    ]
    static let menus = ["primary", "button.background", "button.hover", "button.text", "border.strong", "chip.selected", "text.on_selected"]
    static let controls = [
        "primary", "chip.selected", "text.on_selected", "banner.info.background", "banner.info.border",
        "banner.warning.background", "banner.warning.border", "banner.danger.background", "banner.danger.border",
        "banner.success.background", "banner.success.border", "status.info", "status.warning", "status.danger", "status.success",
    ]
}
