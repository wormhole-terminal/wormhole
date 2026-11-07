// The chip over a terminal: centred on the top edge, resting `restGap` below it. It drops in on a spring (400 /
// 30) from above the edge and goes back up along ease-in; Reduce Motion fades it in place instead. The
// terminal's own bounds are the clip: nothing of the chip shows above the edge it comes from. A chip that only
// reports (a copy) takes no clicks, so the terminal under it keeps them.

import DesignSystem
import SwiftUI

public struct NoticeChipHost: View {
    let center: NoticeCenter
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(center: NoticeCenter) { self.center = center }

    public var body: some View {
        ZStack(alignment: .top) {
            if let notice = center.shown, !center.leaving {
                NoticeChip(notice: notice, center: center)
                    .padding(.top, NoticeTiming.restGap)
                    .padding(.horizontal, NoticeChipMetrics.windowMargin)
                    .allowsHitTesting(center.isClickable)
                    .transition(reduceMotion ? .opacity : .offset(y: -(NoticeChipMetrics.height + NoticeTiming.restGap)))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .clipped()
        .animation(animation, value: center.leaving)
        .animation(animation, value: center.shown == nil)
        .onChange(of: reduceMotion, initial: true) { center.reduceMotion = reduceMotion }
        // VoiceOver says the same words when the chip appears (the GPUI app announces them).
        .onChange(of: center.serial) { if let words = center.shown?.words { announce(words) } }
        .allowsHitTesting(center.isClickable)
    }

    private var animation: Animation {
        if reduceMotion { return .easeOut(duration: center.leaving ? NoticeTiming.fadeOut : NoticeTiming.fadeIn) }
        if center.leaving { return .timingCurve(0.3, 0, 1, 1, duration: NoticeTiming.up) }
        return .spring(Spring(mass: 1, stiffness: NoticeTiming.dropStiffness, damping: NoticeTiming.dropDamping))
    }

    private func announce(_ words: String) {
        var text = AttributedString(words)
        text.accessibilitySpeechAnnouncementPriority = .default
        AccessibilityNotification.Announcement(text).post()
    }
}
