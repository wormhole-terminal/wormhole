// The table with one action's keys changed: the jump-to-waiting key is the person's to record.

extension Keymap {
    public func replacingKeys(of id: ActionID, with keys: [KeySpec]) -> Keymap {
        var entries = self.entries
        guard let at = entries.firstIndex(where: { $0.id == id }) else { return self }
        entries[at].keys = keys
        return Keymap(entries: entries, systemKeys: systemKeys, layout: layout)
    }
}
