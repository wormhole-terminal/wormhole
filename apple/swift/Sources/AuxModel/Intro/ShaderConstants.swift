// The moments the shader needs, as the `constant float` lines it is built on (`shader_constants`): the model owns
// them, so the shader files have no copy that could drift.

import Foundation

public enum ShaderConstants {
    public static var source: String {
        let constants: [(String, Float)] = [
            ("FTL_JUMP", IntroStyle.ftlJump), ("FTL_EXIT", IntroStyle.ftlExit),
            ("WORMHOLE_GATHER", Wormhole.gather), ("WORMHOLE_JUMP", Wormhole.jump),
            ("WORMHOLE_EXIT", Wormhole.exit), ("WORMHOLE_ARRIVE", Wormhole.arrive),
            ("VOYAGE_LAND", Voyage.land), ("VOYAGE_DISSOLVE", Voyage.dissolve),
            ("VOYAGE_CANVAS", Voyage.iconCanvas), ("VOYAGE_STROKE", Voyage.iconStroke),
            ("VOYAGE_APEX_Y", Voyage.apexY), ("VOYAGE_ARM_X", Voyage.armX), ("VOYAGE_ARM_Y", Voyage.armY),
            ("VOYAGE_AURORA", Voyage.aurora ? 1 : 0),
        ]
        // `\(Float)` prints the shortest round-trip form, and always with a point (`4.6`, `1024.0`).
        return constants.map { "constant float \($0.0) = \($0.1);\n" }.joined()
    }
}
