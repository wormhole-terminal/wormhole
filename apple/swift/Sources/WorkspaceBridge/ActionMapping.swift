// The window's actions as the core's (`WormholeBindings.Action`): one case each, the ids widened to the core's. The
// drag's targets and landings are in `DropMapping.swift`, the panel and layout names in `NameMapping.swift`.

import WorkspaceModel
import WormholeBindings

extension WormholeBindings.Action {
    init(_ action: WorkspaceAction) {
        switch action {
        case .select(let id): self = .select(Int64(id))
        case .selectStep(let delta): self = .selectStep(Int32(delta))
        case .selectNumber(let number): self = .selectNumber(UInt32(number))
        case .newTerminal, .newTerminalIn: self = .newTerminal
        case .newTerminalInGroup(let group): self = .newTerminalInGroup(group)
        case .newTerminalInNewGroup: self = .newTerminalInNewGroup
        case .close(let id): self = .close(Int64(id))
        case .rename(let id, let name): self = .rename(id: Int64(id), name: name)
        case .togglePin(let id): self = .togglePin(Int64(id))
        case .setAgentHidden(let id, let hidden): self = .setAgentHidden(id: Int64(id), hidden: hidden)
        case .setForgetText(let id, let forget): self = .setForgetText(id: Int64(id), forget: forget)
        default: self = Self.group(action) ?? Self.layout(action)
        }
    }

    /// The group's actions and the drags that move things in the list.
    private static func group(_ action: WorkspaceAction) -> Self? {
        switch action {
        case .newGroup(let members): .newGroup(members: members.map(Int64.init))
        case .recoverGroup(let members): .recoverGroup(members: members.map(Int64.init))
        case .renameGroup(let group, let name): .renameGroup(group: group, name: name)
        case .toggleGroup(let group): .toggleGroup(group)
        case .setGroupPinned(let group, let pinned): .setGroupPinned(group: group, pinned: pinned)
        case .ungroup(let group): .ungroup(group)
        case .addToGroup(let id, let group): .addToGroup(id: Int64(id), group: group)
        case .removeFromGroup(let id): .removeFromGroup(Int64(id))
        case .setGroupColor(let group, let colour): .setGroupColor(group: group, color: colour.map(WormholeBindings.GroupColor.init))
        case .setGroupCustomColor(let group, let red, let green, let blue):
            .setGroupCustomColor(group: group, r: red, g: green, b: blue)
        case .setGroupFolder(let group, let folder): .setGroupFolder(group: group, folder: folder)
        case .setOnlyInFolder(let group, let on): .setOnlyInFolder(group: group, on: on)
        case .declineGroupFolder(let group): .declineGroupFolder(group)
        case .closeGroup(let group): .closeGroup(group)
        case .moveGroupToWindow(let group): .moveGroupToWindow(group)
        case .dropTerminal(let id, let over): .dropTerminal(dragged: Int64(id), over: Over(over))
        case .dropGroup(let group, let over): .dropGroup(group: group, over: Over(over))
        case .moveTerminal(let id, let landing, let leaves, let mates):
            .moveTerminalAs(id: Int64(id), landing: WormholeBindings.Landing(landing), leaveSplit: leaves, withMates: mates)
        case .moveGroup(let group, let landing): .moveGroup(group: group, landing: WormholeBindings.Landing(landing))
        default: nil
        }
    }

    /// Splits, panes and the window's own switches.
    private static func layout(_ action: WorkspaceAction) -> Self {
        switch action {
        case .splitRight(let id): .splitBeside(anchor: Int64(id), axis: .row)
        case .splitDown(let id): .splitBeside(anchor: Int64(id), axis: .column)
        case .toggleZoom: .toggleZoom
        case .joinSplit(let id, let target, let zone): .joinSplit(id: Int64(id), target: Int64(target), zone: WormholeBindings.Zone(zone))
        case .movePane(let id, let target, let zone):
            .movePane(id: Int64(id), target: Int64(target), zone: WormholeBindings.Zone(zone))
        case .focusDirection(let direction): .focusDirection(WormholeBindings.Direction(direction))
        case .leaveSplit(let id): .leaveSplit(Int64(id))
        case .showBeside(let id, let other): .showBeside(id: Int64(id), other: Int64(other))
        case .applyLayout(let preset): .applyLayout(WormholeBindings.Preset(preset))
        case .dragDivider(let path, let index, let position): .dragDivider(path: path, index: index, position: Float(position))
        case .equalizeDivider(let path, let index): .equalizeDivider(path: path, index: index)
        case .toggleSidebar: .toggleSidebar
        case .togglePanel: .togglePanel
        case .showPanelTab(let tab): .showPanelTab(TabId(tab))
        case .search(let text): .search(text)
        case .showOverlay(let overlay): .showOverlay(overlay.map(WormholeBindings.Overlay.init))
        case .setWindowWidth(let width): .setWindowWidth(Float(width))
        case .setPaneArea(let width, let height): .setPaneArea(width: Float(width), height: Float(height))
        case .dragEdge(let edge, let extent): .dragEdge(edge: WormholeBindings.Edge(edge), extent: Float(extent))
        case .resetEdge(let edge): .resetEdge(WormholeBindings.Edge(edge))
        case .setFeature(let key, let on): .setFeature(feature: FeatureId(key), on: on)
        case .setTitleRow(let keys): .setTitleRow(keys.compactMap(WormholeBindings.TitlePart.init(key:)))
        case .setNumbersFollowList(let follow): .setNumbersFollowList(follow)
        case .setChangeRules(let plain, let shared):
            .setChangeRules(
                ChangeRules(
                    plain: plain == "any" ? PlainChanges.any : plain == "none" ? PlainChanges.none : PlainChanges.finishing,
                    shared: shared == "none" ? SharedChanges.none : SharedChanges.each))
        default: .newTerminal
        }
    }
}
