// --- Aurora --------------------------------------------------------------------------------------

// What blooms behind the window in Aurora: curtains of aurora that flow out to the screen's edges,
// over the wormhole of `wormhole.metal`.

// One aurora curtain: a sheet of light hanging in the sky, seen from below. It runs along a hem, a
// slow curve with a bright, sharp lower border, and soft rays climb from it, longer where they are
// brighter, fading to the tips. The curtain doesn't fill the sky: it has ends, and bright stretches
// that come and go along it. The rays are straight lines through a point far above the screen, so
// they lean away from its middle as a curtain overhead does, and the pattern of them slides along
// and shifts as it goes: never the same stripes twice.
struct Curtain {
    float hem;     // the hem's height at rest, as a fraction of the screen's, from the top
    float sway;    // how far the hem sweeps, as a fraction of the screen's height
    float height;  // how far the rays climb, as a fraction of the screen's height
    float rays;    // how many rays there are, across the fan
    float flow;    // how fast the pattern slides along, rays a second
    float seed;
    float gain;    // brightness
    float center;  // where along the fan it hangs, -1 to 1 across the screen
    float span;    // how far it reaches either side of that
    float lean;    // which way the rays lean, as a fraction of the screen's width
    float3 low;    // colour at the hem
    float3 body;   // and above it
    float3 tip;    // and where the rays end
};

// The curtain's light at `sc`, points from the screen's top left, `s` seconds in, risen by `lift`.
static float3 curtain(float2 sc, Curtain k, float s, float lift, constant Uniforms& u) {
    float H = u.size.y;
    float2 vanish = float2((0.5 + k.lean) * u.size.x, -4.0 * H);
    // Which ray this is: -1 to 1 across the screen along its bottom edge, whatever its shape.
    float a = (sc.x - vanish.x) / max(sc.y - vanish.y, 1.0) * 10.0 * H / u.size.x;

    // The hem sweeps across the screen in a slow wave, a smaller one on top and a drift.
    float wave = 0.7 * sin(a * 1.9 + s * 0.17 + k.seed) + 0.22 * sin(a * 4.4 - s * 0.29 + k.seed * 2.1)
        + 0.08 * sin(a * 9.0 + s * 0.5 + k.seed * 3.7);
    float drift = 1.2 * (fbm3(float2(a * 1.3 + k.seed, s * 0.06)) - 0.5);
    float hem = (k.hem + k.sway * (wave + drift) - 0.3 * lift) * H;
    // Where the hem climbs steeply the sheet is seen edge on, and brighter.
    float fold = 1.0 + 1.5 * saturate(abs(dfdx(hem)) * 0.8);

    float up = hem - sc.y;
    if (up < -50.0 || up > k.height * H * 3.0) return float3(0.0);

    // Where it hangs: ends that drift, and bright stretches that come and go.
    float middle = k.center + 0.35 * sin(s * 0.33 + k.seed);
    float ends = smoothstep(k.span, k.span * 0.4, abs(a - middle));
    float patch = smoothstep(0.3, 0.7, fbm3(float2(a * 1.1 + s * 0.12 + k.seed, s * 0.15 + k.seed)));
    float presence = ends * (0.25 + 0.75 * patch);
    if (presence < 0.003) return float3(0.0);

    // The rays: soft stripes that slide along, finer ones that move the other way and broad groups
    // of them that make them uneven. They waver as they climb, never ruler straight.
    float waver = noise(float2(up * 0.004 - s * 0.2, k.seed * 2.0)) - 0.5;
    float along = a * k.rays * (u.size.x / 1440.0) + 1.5 * waver;
    float slide = s * k.flow;
    float n0 = noise(float2(along * 0.45 + slide * 0.3 + 11.0, k.seed));
    float n1 = noise(float2(along + slide, k.seed + 3.0 + s * 0.2));
    float n2 = noise(float2(along * 2.3 - slide * 1.7, k.seed + 8.0 - s * 0.3));
    float ray = smoothstep(0.2, 0.8, (0.45 * n1 + 0.3 * n2 + 0.45 * n0) / 1.2);

    float reach = k.height * H * (0.55 + 0.7 * ray) * (0.7 + 0.5 * patch) * (1.0 - 0.45 * lift);
    float edge = smoothstep(-4.0, 9.0, up);
    // Above the hem the light fades up from it; below it, a faint fringe.
    float profile = mix(0.2 * exp(min(up, 0.0) / 16.0), exp(-max(up, 0.0) / reach), edge);
    // A soft haze between the rays, so the curtain has body.
    float haze = 0.12 * edge * exp(-max(up, 0.0) / (reach * 2.2));
    // Ripples climbing the rays.
    float shimmer = 0.88 + 0.12 * sin(up * 0.025 - s * 2.4 + ray * 6.0);
    float brightness = (profile * (0.4 + 0.9 * ray) * shimmer + haze) * presence * fold * k.gain;

    // Hot at the hem, the body's colour above, the tip's where the rays end, a slow shift between.
    float height01 = saturate(up / (k.height * H));
    float3 col = mix(k.low, k.body, smoothstep(0.0, 0.12, height01));
    float shift = fbm3(float2(a * 0.9 + k.seed * 1.3, s * 0.05));
    col = mix(col, k.tip, saturate(smoothstep(0.25, 0.9, height01) + 0.35 * (shift - 0.4)));

    // A few small motes drifting up, only where it is bright.
    float motes = dust(float2(sc.x / 14.0, (sc.y - s * 30.0) / 14.0), brightness * 0.06);
    return (col * brightness + mix(col, float3(1.0), 0.6) * motes * 0.35) * pow(1.0 - lift, 1.5);
}

// The three curtains, back to front, over the screen: each nearer one a little lower, brighter and
// finer. `q` is from the screen's centre, and the bloom flows out from there behind the portal's rim.
static float3 aurora_curtains(float2 sc, float2 q, WormholeClock c, constant Uniforms& u) {
    float3 white = float3(1.0);
    // In the order of `Curtain`'s fields: hem, sway, height, rays, flow, seed, gain, center, span,
    // lean, then the colours at the hem, above it and at the tips.
    Curtain back = {
        0.50, 0.17, 0.50, 34.0, 0.35, 1.3, 0.55, -0.45, 1.0, -0.08,
        mix(u.c1.rgb, white, 0.35), mix(u.c1.rgb, u.c3.rgb, 0.4), u.c3.rgb,
    };
    Curtain mid = {
        0.66, 0.16, 0.45, 42.0, -0.5, 4.1, 0.85, 0.5, 0.95, 0.05,
        mix(u.c2.rgb, white, 0.5), u.c2.rgb, mix(u.c1.rgb, u.c3.rgb, 0.6),
    };
    Curtain near = {
        0.86, 0.12, 0.38, 55.0, 0.7, 7.7, 1.1, 0.05, 1.2, 0.0,
        mix(u.c2.rgb, white, 0.65), mix(u.c2.rgb, u.c1.rgb, 0.4), mix(u.c1.rgb, u.c3.rgb, 0.8),
    };

    // Each curtain blooms in the portal's wake, trailing its rim by its own distance, the back ones
    // last.
    float rim = wormhole_portal_radius(c, u);
    float r = length(q);
    float3 light = float3(0.0);
    light += curtain(sc, back, c.s, c.lift, u) * smoothstep(rim - 330.0, rim - 700.0, r);
    light += curtain(sc, mid, c.s, c.lift, u) * smoothstep(rim - 220.0, rim - 580.0, r);
    light += curtain(sc, near, c.s, c.lift, u) * smoothstep(rim - 120.0, rim - 460.0, r);
    // A soft shoulder on the brightest channel, so where the curtains overlap they glow rather than
    // clip to white, and keep their colour.
    return light / (1.0 + 0.35 * max3(light));
}


static float4 aurora(float2 p, constant Uniforms& u) {
    Wormhole w = wormhole(p, u);
    if (w.clock.since > 0.0) {
        float2 q = p - (u.size * 0.5 - u.center);
        // All of the curtains behind the window.
        w.light += aurora_curtains(p + u.center, q, w.clock, u) * smoothstep(0.0, 4.0, w.wd);
    }
    return wormhole_finish(w, p, u);
}
