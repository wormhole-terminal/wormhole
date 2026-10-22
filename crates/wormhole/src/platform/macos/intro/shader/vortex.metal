// --- Edge Vortex ---------------------------------------------------------------------------------

// The screen sinks into space and the stars stream past towards the window as through a tunnel;
// a band of light runs round the frame, the stars bend round its edges, and space lets go.
static float4 vortex(float2 p, constant Uniforms& u) {
    float t = u.t;
    float d = sd_round_rect(p, u.half_size, u.radius);
    float rad = length(p);
    float ang = atan2(p.y, p.x);
    float cover = smoothstep(0.0, 0.14, t) * (1.0 - smoothstep(0.74, 1.0, t)) * 0.95;
    float rush = smoothstep(0.05, 0.5, t) * (1.0 - smoothstep(0.6, 0.85, t));

    // Lensing round the frame.
    float size = min(u.half_size.x, u.half_size.y);
    float einstein = size * 0.6 * cover;
    float2 n = normalize(p + 1e-4);
    float2 source = p - n * (einstein * einstein) / (max(d, 0.0) + size);
    source = rotate(source, 1.2 * exp(-max(d, 0.0) / 260.0) * cover - u.time * 0.15);
    float3 space = sky(source + float2(u.time * 6.0, 0.0), 0.0, 0.9, u);

    // Streaks: stars on a polar grid rushing inward, stretched along their way.
    float3 streaks = float3(0.0);
    float travel = u.time * 3.0 + u.time * u.time * 1.6;
    for (int i = 0; i < 2; i++) {
        float lanes = 90.0 + 70.0 * float(i);
        float2 g = float2(ang / (2.0 * M_PI_F) * lanes, log(max(rad, 1.0)) * 6.0 + travel * (1.0 + 0.4 * float(i)));
        float2 cell = floor(g);
        float2 f = fract(g);
        float h = hash21(cell + 13.0 * float(i));
        if (h < 0.3) {
            float lane = exp(-pow((f.x - 0.5) * 7.0, 2.0));
            float tail = smoothstep(0.0, 0.7, f.y) * smoothstep(1.0, 0.85, f.y);
            streaks += mix(u.c1.rgb, float3(1.0), 0.5 + 0.5 * h / 0.3) * lane * tail;
        }
    }
    float3 e = streaks * rush * smoothstep(0.0, 30.0, d) * 0.9;

    // The band of light round the frame.
    float head = -M_PI_F / 2.0 + smoothstep(0.15, 0.85, t) * 2.0 * M_PI_F * 1.5;
    float behind = fract((head - ang) / (2.0 * M_PI_F));
    float comet = max(exp(-behind * 3.0), exp(-(1.0 - behind) * 40.0));
    float band = smoothstep(0.12, 0.25, t) * (1.0 - smoothstep(0.78, 1.0, t));
    float flame = fbm(rotate(p, 2.0 * exp(-max(d, 0.0) / 40.0) - u.time * 3.0) * 0.03);
    float halo = exp(-max(d, 0.0) / (12.0 + 40.0 * flame * comet)) * step(0.0, d);
    float3 hot = mix(u.c1.rgb, float3(1.0), 0.7);
    e += mix(u.c2.rgb, u.c1.rgb, comet) * halo * (0.3 + 2.0 * comet) * (0.55 + 0.9 * flame) * band;
    e += hot * exp(-abs(d) / 1.3) * (0.35 + 1.6 * comet) * band;

    return over_window(compose(space, cover, e), d, u);
}
