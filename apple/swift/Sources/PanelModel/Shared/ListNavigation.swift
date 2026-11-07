// Arrow-key walking through a list: no wrap-around, and the first press from nothing lands on an end.

public enum ListNavigation {
    public static func move<ID: Equatable>(in ids: [ID], from current: ID?, by step: Int) -> ID? {
        guard !ids.isEmpty else { return nil }
        guard let current, let index = ids.firstIndex(of: current) else {
            return step >= 0 ? ids.first : ids.last
        }
        return ids[min(max(index + step, 0), ids.count - 1)]
    }
}
