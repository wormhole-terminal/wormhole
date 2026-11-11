// Whether a row shows: a choice that does nothing while the switch it belongs to is off is not on the page, nor
// found by search (`Entry::is_shown` in the Rust index, whose `needs` the dump writes as conditions).

extension SettingsSchema.Entry {
    public func isShown(values: [String: JSONValue]) -> Bool {
        (visibleWhen ?? []).allSatisfy { values[$0.key] == $0.equals }
    }
}
