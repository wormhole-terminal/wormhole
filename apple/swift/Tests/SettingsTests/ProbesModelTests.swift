import SettingsModel
import XCTest

/// The sheet's rules: which permissions are left on decide whether approving needs Touch ID, nothing changes while the
/// system asks, and a Touch ID the person cancels changes nothing.
@MainActor
final class ProbesModelTests: XCTestCase {
    private actor Fake: ProbesBackend {
        var approvals: [Set<Int>] = []
        var answer = ProbeApproval.cancelled
        func set(_ answer: ProbeApproval) { self.answer = answer }
        func list() async -> ProbeList { ProbeList(rows: [ProbeRow(id: "acme", title: "Acme", state: .off, stateWord: "Off", canApprove: true)]) }
        func sheet(_ id: String) async throws -> ProbeSheet {
            ProbeSheet(id: id, title: "Turn on Acme", permissions: [ProbePermission(id: 0, words: "Notify"), ProbePermission(id: 1, words: "Run git", touchID: true)])
        }
        func needsTouchID(_ id: String, granted: Set<Int>) async -> Bool { granted.contains(1) }
        func approve(_ id: String, granted: Set<Int>) async -> ProbeApproval {
            approvals.append(granted)
            return answer
        }
        func turnOff(_ id: String) async throws {}
    }

    func testTouchIDFollowsThePermissionsLeftOnAndCancelChangesNothing() async throws {
        let fake = Fake()
        let model = ProbesModel(backend: fake)
        await model.openSheet("acme")
        XCTAssertEqual(model.open?.granted, [0, 1])
        XCTAssertEqual(model.open?.needsTouchID, true)

        await model.toggle(try XCTUnwrap(model.open?.sheet.permissions.last))
        XCTAssertEqual(model.open?.granted, [0])
        XCTAssertEqual(model.open?.needsTouchID, false, "a click is enough once the Touch ID level is off")

        await model.approve()
        XCTAssertNotNil(model.open, "cancelled: the sheet stays as it was")
        XCTAssertEqual(model.open?.busy, false)
        XCTAssertEqual(model.open?.error, nil)
        let approvals = await fake.approvals
        XCTAssertEqual(approvals, [[0]], "exactly the permissions left on")

        await fake.set(.approved)
        await model.approve()
        XCTAssertNil(model.open)
    }
}
