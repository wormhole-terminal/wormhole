// --- Flying through space ------------------------------------------------------------------------

// What the two jumps (FTL and Aurora) share: noise round a point and stars streaking out of the
// screen's centre.

// Noise round a point with no seam where atan2's angle wraps: two samples a turn apart, blended
// so both ends of the range read the same.
static float polar_fbm(float ang, float y, float k) {
    float a = fbm(float2(ang * k, y));
    float b = fbm(float2((ang - 2.0 * M_PI_F) * k, y));
    return mix(a, b, (ang + M_PI_F) / (2.0 * M_PI_F));
}

// Stars flying out from the centre of the screen as you travel into it: on a grid in angle and
// log distance, moving outward, each a streak as long as the speed, brighter as it nears you.
static float3 warp_streaks(float2 q, float zoom, float speed, constant Uniforms& u) {
    float r = length(q);
    float ang = atan2(q.y, q.x);
    float3 col = float3(0.0);
    float len = 0.06 + 0.9 * clamp(speed, 0.0, 1.0);
    for (int i = 0; i < 3; i++) {
        float lanes = 140.0 + 90.0 * float(i);
        float2 g = float2(ang / (2.0 * M_PI_F) * lanes, log(max(r, 1.0)) * 4.0 - zoom * (1.0 + 0.35 * float(i)));
        // Each lane its own phase, or the streaks would line up into rings.
        g.y += hash21(float2(floor(g.x), 41.0 * float(i))) * 7.3;
        float2 cell = floor(g);
        float2 f = fract(g);
        float h = hash21(cell + 23.0 * float(i));
        if (h > 0.11) continue;
        float width = r * 2.0 * M_PI_F / lanes;
        float off = (hash21(cell + 3.3) - 0.5) * 0.6;
        float lane = exp(-pow((f.x - 0.5 - off) * width / 1.1, 2.0));
        // The head leads outward (f.y near 1); the tail trails back towards the centre.
        float tail = smoothstep(1.0 - len, 1.0, f.y) * smoothstep(1.0, 0.96, f.y);
        float3 tint = mix(mix(u.c1.rgb, u.c3.rgb, hash21(cell + 6.1)), float3(1.0), 0.55);
        col += tint * lane * tail * (0.45 + 1.4 * pow(hash21(cell + 8.8), 4.0));
    }
    return col * smoothstep(0.0, 120.0, r);
}

// The plasma tunnel of the jumps: mist flowing outward past the screen's edges, streaked along its way,
// in the theme's blue and violet. `r`, `ang` and `rn` (r over the screen's half diagonal) are the
// point's polar place about the screen's centre. rgb is its light; a is how strong the strands are, which
// the jump's shimmer follows.
static float4 tunnel_mist(float r, float ang, float rn, float zoom, constant Uniforms& u) {
    float3 hot = mix(u.c1.rgb, float3(1.0), 0.5);
    float y = log(max(r, 1.0)) * 1.4 - zoom * 0.7;
    float mist = polar_fbm(ang, y * 1.2, 2.5);
    float strands = polar_fbm(ang, y * 0.6 + 5.0, 9.0);
    float walls = smoothstep(0.08, 0.9, rn);
    float3 violet = mix(u.c1.rgb, u.c3.rgb, 0.2 + 0.6 * mist) * 0.9;
    float3 tunnel = violet * walls * (0.12 + 1.9 * pow(mist, 2.2)) * (0.35 + 1.3 * strands);
    tunnel += hot * pow(strands, 6.0) * walls * 3.5;
    return float4(tunnel, strands);
}
