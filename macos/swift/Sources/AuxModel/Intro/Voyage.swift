// Voyage: the wormhole run with a ship flown through it. The ship is the `>` of the app icon turned to `^`, big in
// front of the camera: it rises into the funnel, takes the jump, swoops across the window when the portal opens
// and docks on the logo of the intro window's page, where the icon's own chevron is. The page comes in from there.
// Moments and the icon's drawing are core's (`onboarding::intro::voyage`).
//
//   0 - launch - jump ------- cruise ------- exit - swoop - dock - land - page - lift - end

import Foundation

public enum Voyage {
    /// Whether the aurora arms bloom behind the window once the ship has docked: off, as in the GPUI app.
    public static let aurora = false
    public static let seconds: Float = aurora ? 10 : 8.2
    /// When the ship leaves the cruise for its swoop: as the portal opens.
    public static let swoop: Float = Wormhole.exit + 0.1
    /// When it has reached the logo and settles onto it.
    public static let dock: Float = land - 0.4
    /// When it has docked: the icon's chevron is under it, and the rest of the icon fades in.
    public static let land: Float = 6.2
    /// How long it takes to dissolve into the logo once docked.
    public static let dissolve: Float = 0.35
    public static let lift: Float = aurora ? 8.7 : 7.2

    /// The app icon's canvas, in its own units, and its `>`: a round-capped polyline with the cursor's width.
    public static let iconCanvas: Float = 1024
    public static let iconStroke: Float = 62
    /// The middle of the chevron's own box on the canvas, and its extent with the stroke.
    public static let chevronCentre: SIMD2<Float> = [438.4, 512]
    public static let chevronExtent: SIMD2<Float> = [181.6, 292]
    /// How much of the image's box the icon's canvas fills: the artwork sits on the macOS grid, 824 of 1024.
    public static let iconBody: Float = 824 / iconCanvas
    /// The ship's drawing in canvas units about the chevron's centre, turned to point up.
    public static let apexY: Float = -59.8
    public static let armX: Float = 115
    public static let armY: Float = 59.8

    /// How many past positions the comet trail is drawn through, and how far apart in time.
    public static let trailPoints = 24
    public static let trailStep: Float = 0.03
}

/// Where the ship lands: the logo's box on screen, in points from the screen's top left.
public struct Landing: Equatable, Sendable {
    /// The middle of the logo's box, and its side.
    public var centre: SIMD2<Float>
    public var size: Float

    public init(centre: SIMD2<Float>, size: Float) { (self.centre, self.size) = (centre, size) }

    /// The side of the icon's canvas on screen: the artwork's box, inside the logo's. The ship's size means this.
    public var canvas: Float { size * Voyage.iconBody }

    /// Where the icon's chevron is on screen: its box's middle, which sits left of the icon's.
    public var chevron: SIMD2<Float> {
        let k = canvas / Voyage.iconCanvas
        return centre + (Voyage.chevronCentre - Voyage.iconCanvas / 2) * k
    }

    /// The landing that puts the ship's chevron's middle at `chevron` with its height at `height`: for a target
    /// that is not an image of the icon, like the terminal cell the welcome program draws its `>` in.
    public static func onChevron(_ chevron: SIMD2<Float>, height: Float) -> Landing {
        let canvas = height * Voyage.iconCanvas / Voyage.chevronExtent.y
        let k = canvas / Voyage.iconCanvas
        return Landing(
            centre: chevron - (Voyage.chevronCentre - Voyage.iconCanvas / 2) * k, size: canvas / Voyage.iconBody)
    }
}
