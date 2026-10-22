// --- The wormhole --------------------------------------------------------------------------------

// The run that Aurora and Tentacles share: a particle wormhole at the screen's centre, over the
// desktop, and the portal it opens onto.
//
//   gather   glittering particles drift in from round the screen and draw together into a funnel
//            of particle mist round a dark throat;
//   jump     a flash, space spreads out of the throat over the screen, and the stars stretch into
//            streaks flying out of it;
//   exit     they snap back and spring; the throat flashes and opens as a portal, an iris of light
//            that pushes space out to the screen's edges and leaves the desktop behind it;
//   bloom    the window arrives, and in the portal's wake the style's own light blooms out from
//            behind it to the screen's edges (`aurora()`, `tentacles()`);
//   lift     it rises away and fades, the last glitter with it.
//
// The moments are core's (WORMHOLE_*, in seconds; the run's length and lift are the style's, `u.run`), and
// every envelope comes from `wormhole_clock`.

// Where the run is: the envelopes the effect is made of, from the seconds in.
struct WormholeClock {
    float s;       // seconds in
    float appear;  // the first particles showing
    float gather;  // 0 to 1: the funnel drawing together round its throat
    float open;    // 0 to 1: the throat opening
    float jump;    // 0 to 1 and back: the streaks, from the jump to the drop out
    float flash;   // the flashes at the jump and at the drop out
    float after;   // 0 to 1: the drop out has happened
    float since;   // seconds since the drop out, 0 before it
    float bounce;  // space's spring back after the drop out
    float iris;    // 0 to 1: how far the portal has opened
    float lift;    // 0 to 1: the bloom rising away
};

static WormholeClock wormhole_clock(float s, constant Uniforms& u) {
    WormholeClock c;
    c.s = s;
    c.appear = smoothstep(0.0, 0.25, s);
    // Quick at first, settling: the funnel is half drawn in before the first half second is over.
    c.gather = 1.0 - pow(1.0 - saturate(s / WORMHOLE_GATHER), 3.0);
    c.open = smoothstep(0.05, WORMHOLE_GATHER, s);
    c.jump = smoothstep(WORMHOLE_JUMP - 0.22, WORMHOLE_JUMP + 0.44, s)
        * (1.0 - smoothstep(WORMHOLE_EXIT - 0.05, WORMHOLE_EXIT + 0.17, s));
    c.flash = 0.5 * exp(-pow((s - WORMHOLE_JUMP) / 0.1, 2.0)) + exp(-pow((s - WORMHOLE_EXIT) / 0.12, 2.0));
    c.after = smoothstep(WORMHOLE_EXIT - 0.03, WORMHOLE_EXIT + 0.05, s);
    c.since = max(s - WORMHOLE_EXIT, 0.0);
    c.bounce = exp(-5.0 * c.since) * sin(11.0 * c.since);
    // Quick off the throat, slowing as the rim nears the edges, so the crossing can be seen.
    c.iris = 1.0 - pow(1.0 - saturate((c.since - 0.04) / 1.0), 2.4);
    c.lift = smoothstep(u.run.y, u.run.x, s);
    return c;
}

// Glittering dust: a fine grid of points in `g`'s space, lit where `density` says. Since the grid
// lives in the flow's own space, the dust moves with whatever `g` follows.
static float dust(float2 g, float density) {
    float2 cell = floor(g);
    float2 f = fract(g) - 0.5;
    float h = hash21(cell);
    float2 o = (float2(hash21(cell + 1.7), hash21(cell + 4.1)) - 0.5) * 0.5;
    return step(h, density) * smoothstep(0.45, 0.05, length(f - o)) * (0.5 + 0.5 * hash21(cell + 3.7));
}

// --- The funnel ----------------------------------------------------------------------------------

// The throat's radius, points: it opens as the funnel gathers and swells in the jump. At the drop
// out it is about WORMHOLE_THROAT_AT_EXIT, which is where the portal starts.
constant float WORMHOLE_THROAT_AT_EXIT = 136.0;

static float wormhole_throat(WormholeClock c) {
    return max(42.0 * c.open, 2.0) * (1.0 + 2.5 * c.jump);
}

// The funnel's light (rgb) and the dark of its throat (a), at `q` from the screen's centre: uneven
// strands of particle mist in log depth, flowing with `travel` (inward while it gathers), spread
// thin over the screen at first and drawn together into the funnel.
static float4 wormhole_funnel(float2 q, WormholeClock c, constant Uniforms& u) {
    float r = length(q);
    float ang = atan2(q.y, q.x);
    float far = length(u.size) * 0.5;
    float3 ice = mix(u.c1.rgb, float3(1.0), 0.55);
    float3 tint = mix(u.c1.rgb, u.c3.rgb, 0.3);
    // Gone by the time the portal is a breath old.
    float on = c.appear * (1.0 - smoothstep(WORMHOLE_EXIT, WORMHOLE_EXIT + 0.25, c.s));

    float y = log(max(r, 1.0)) * 3.0 - u.travel;
    float curl = polar_fbm(ang, y * 0.4, 1.0) - 0.5;
    float strands = pow(polar_fbm(ang + 0.8 * curl, y * 0.25, 7.0), 3.0) * 2.2;
    float wisps = polar_fbm(ang, y * 1.4 + 4.0, 3.0);
    float density = strands * (0.4 + 0.9 * wisps) * (0.6 + 0.4 * c.gather);
    float throat = wormhole_throat(c);
    float reach = mix(mix(far * 1.1, far * 0.32, c.gather), far * 0.9, c.jump);
    float shape = smoothstep(throat * 0.9, throat * 1.5, r) * exp(-max(r - throat, 0.0) / reach);
    float lip = exp(-max(r - throat, 0.0) / (throat * 0.8 + 10.0)) * c.gather;
    float glitter = dust(float2(ang * 140.0, y * 26.0), density * 0.8)
        + dust(float2(ang * 300.0 + 7.0, y * 60.0), density * 0.55) * 0.7;
    // Voyage swaps the fine glitter for smooth mist: less of the grain, more of the soft density.
    float calm = u.mist;
    float3 mist = mix(tint, ice, lip * 0.5)
        * (glitter * 1.4 * (1.0 - 0.75 * calm) + density * (0.1 + 0.35 * lip) * (1.0 + 1.5 * calm)) * shape;

    // The throat is dark, even over the desktop.
    float hole = smoothstep(throat * 1.05, throat * 0.7, r) * smoothstep(0.15, WORMHOLE_GATHER, c.s);
    return float4(mist * on * (1.0 - 0.35 * c.jump - 0.6 * calm * c.jump), hole * on * 0.92);
}

// --- Space ---------------------------------------------------------------------------------------

// The stars the wormhole flies through: still ones either side of the jump, streaks during it, a
// flash at either end of it; after the drop out they spring and are pushed out by the portal.
static float3 wormhole_space(float2 q, WormholeClock c, constant Uniforms& u) {
    float3 ice = mix(u.c1.rgb, float3(1.0), 0.55);
    float push = 1.0 + 0.1 * c.bounce - 0.35 * c.iris;
    float3 space = sky(q / push, 6.0, 0.55, u) * (1.0 - c.jump);
    if (c.jump > 0.0) {
        space += warp_streaks(q, u.travel * 0.6, clamp(u.speed / 13.0, 0.0, 1.0), u) * c.jump;
    }
    return space + ice * c.flash * 0.07;
}

// --- The portal ----------------------------------------------------------------------------------

// How wide the portal is, points from the screen's centre: from the throat out past the corners.
static float wormhole_portal_radius(WormholeClock c, constant Uniforms& u) {
    return mix(WORMHOLE_THROAT_AT_EXIT, length(u.size) * 0.65 + 160.0, c.iris);
}

// The drop out opens the throat as a portal: inside its rim the desktop is back, outside it is
// still space, and the rim, a hot ring that thins as it widens, pushes the stars out to the edges.
// Returns the light of the rim and the flash (rgb) and how much of the screen is still space (a).
static float4 wormhole_portal(float2 q, WormholeClock c, constant Uniforms& u) {
    float r = length(q);
    float ang = atan2(q.y, q.x);
    float3 ice = mix(u.c1.rgb, float3(1.0), 0.55);
    float3 hot = mix(u.c2.rgb, float3(1.0), 0.7);

    float radius = wormhole_portal_radius(c, u);
    float rim = 10.0 + 0.05 * radius;
    float inside = smoothstep(radius + rim, radius - rim, r);
    float fade = pow(1.0 - c.iris, 1.5) * c.after;
    float ring = exp(-pow((r - radius) / rim, 2.0));
    float outer = exp(-max(r - radius, 0.0) / (0.2 * radius + 40.0)) * step(radius, r);

    float3 light = mix(ice, hot, 0.5) * ring * fade * 2.0 + ice * outer * fade * 0.3;
    // Glitter thrown out ahead of the rim, the funnel's particles carried out with it.
    float sparks = dust(float2(ang * 110.0, r / 3.5 - c.since * 90.0), 0.55 * exp(-abs(r - radius) / 70.0));
    light += hot * sparks * fade * 1.4;
    // The flash at the drop out, from the throat.
    light += hot * c.flash * exp(-r / 240.0) * 1.1 * c.after;
    return float4(light, (1.0 - inside) * 0.96);
}

// Three octaves: the curtain's slow shapes don't need more.
static float fbm3(float2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 3; i++) {
        v += a * noise(p);
        p = p * 2.03 + float2(1.7, 9.2);
        a *= 0.5;
    }
    return v / 0.875;
}


// --- The scene -----------------------------------------------------------------------------------

// Everything of the run but the bloom behind the window: the style adds its light to `light` where
// `clock.since` says the bloom has begun, behind the window (times `smoothstep(0.0, 4.0, wd)`), and
// hands it to `wormhole_finish`.
struct Wormhole {
    WormholeClock clock;
    float3 space;  // the stars, for where `cover` is
    float3 light;  // added on top, premultiplied
    float cover;   // how much of the screen is still space
    float wd;      // the distance to the window's edge, negative inside it
};

static Wormhole wormhole(float2 p, constant Uniforms& u) {
    Wormhole w;
    w.clock = wormhole_clock(u.time, u);
    WormholeClock c = w.clock;
    // The funnel is at the screen's centre, wherever the window is.
    float2 q = p - (u.size * 0.5 - u.center);

    w.space = float3(0.0);
    w.light = float3(0.0);
    w.cover = 0.0;

    // The jump, and the portal that ends it: until the rim has left the screen.
    if (c.since < 1.3) {
        float far = length(u.size) * 0.5;
        float r = length(q);
        // Space: none over the desktop until the jump, then spreading out of the throat to fill
        // the screen. After the drop out only what the portal hasn't reached.
        float spread = far * 1.3 * smoothstep(WORMHOLE_JUMP - 0.11, WORMHOLE_JUMP + 0.77, c.s);
        float4 funnel = wormhole_funnel(q, c, u);
        float before = max(smoothstep(spread + 2.0, spread - 220.0, r) * 0.96, funnel.a);
        float4 portal = wormhole_portal(q, c, u);
        w.cover = mix(before, portal.a, c.after);
        w.space = wormhole_space(q, c, u);
        w.light = funnel.rgb + portal.rgb;
        // Voyage's jump flies through the FTL's plasma tunnel, not the funnel's dust.
        if (u.mist > 0.0 && c.jump > 0.0) {
            float4 tunnel = tunnel_mist(r, atan2(q.y, q.x), r / far, u.travel * 0.6, u);
            w.light += tunnel.rgb * c.jump * u.mist * 0.9;
        }
    }

    // The window arrives, its edge catching the light.
    float2 hs = u.half_size * u.win_scale;
    w.wd = sd_round_rect(p, hs, u.radius);
    if (c.since > 0.0) {
        float3 ice = mix(u.c1.rgb, float3(1.0), 0.55);
        float arrive = WORMHOLE_ARRIVE - WORMHOLE_EXIT;
        float arrival = smoothstep(arrive, arrive + 0.4, c.since) * (1.0 - smoothstep(0.9, 2.4, c.since));
        w.light += ice * exp(-abs(w.wd) / 1.4) * arrival * 0.6;
        w.light += mix(u.c2.rgb, ice, 0.4) * exp(-max(w.wd, 0.0) / 60.0) * step(0.0, w.wd) * arrival * 0.25;
    }
    return w;
}

static float4 wormhole_finish(Wormhole w, float2 p, constant Uniforms& u) {
    // Dithered: the soft glows are dark gradients, which 8 bits would show as bands. Only where
    // there is light, so the clear screen stays clear.
    float4 light = compose(w.space, w.cover, w.light);
    float dither = (hash21(p + fract(u.time) * 91.7) - 0.5) / 255.0;
    light.rgb = clamp(light.rgb + dither * step(1.0 / 255.0, light.a), 0.0, light.a);
    return over_window(light, w.wd, u);
}
