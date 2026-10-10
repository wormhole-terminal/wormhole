// --- Tentacles -----------------------------------------------------------------------------------

// What blooms behind the window in Tentacles: arms of aurora that reach out from behind it towards
// the screen's edges, writhing, and draw back in at the end, over the wormhole of `wormhole.metal`.
//
// Each arm is a curve in polar coordinates about the window's centre, `angle(radius)`: a wave
// that travels out along it and swings wider towards the tip, which whips and curls. Round that
// curve it is a ribbon of aurora, broad at its root, hidden behind the window, and thinning to a
// fine point: a sharp bright edge on one side, rays feathering away on the other, bands of light
// rolling out along it. Seen from the pixel, an arm is a distance to that curve, which is cheap
// enough to ask of every arm there is.
struct TentacleLayer {
    int count;
    float width;   // the root's width, points
    float reach;   // how far the arms get, as a fraction of the way to the screen's edge, at least:
                   // past 1 and the heads are off screen once grown
    float sway;    // how far the wave swings the arm sideways, points
    float curl;    // how far the tip itself whips, points
    float speed;   // the wave's speed, radians a second
    float seed;
    float gain;    // brightness
    float3 root;   // colour at the root
    float3 mid;    // and along it
    float3 tip;    // and at the head
};

// How much of a second, quicker wave rides on the first. The arms are kept near radial (the sheer of
// the curve, `tilt`, stays about 1 or less): where an arm runs round the window instead, the rays and
// the fade at its head, which go by radius, would cut along its length.
constant float TENTACLE_OVERTONE = 0.2;

// How far it is from the window's centre to the screen's edge in the direction `dir`.
static float edge_distance(float2 dir, constant Uniforms& u) {
    float2 room = float2(dir.x > 0.0 ? u.size.x - u.center.x : u.center.x,
                         dir.y > 0.0 ? u.size.y - u.center.y : u.center.y);
    float2 steps = room / max(abs(dir), 1e-3);
    return min(steps.x, steps.y);
}

// The light of arm `i` of `layer` at the point `p` (`r` from the window's centre, at angle `phi`).
static float3 tentacle(float2 p, float r, float phi, int i, TentacleLayer layer, WormholeClock c,
                       constant Uniforms& u) {
    float seed = layer.seed;
    float h1 = hash21(float2(float(i) + 1.0, seed));
    float h2 = hash21(float2(float(i) + 1.0, seed + 1.0));
    float h3 = hash21(float2(float(i) + 1.0, seed + 2.0));
    float h4 = hash21(float2(float(i) + 1.0, seed + 3.0));

    // Where it points: spread round the window, nudged, the whole swarm turning slowly.
    float spread = 2.0 * M_PI_F * (float(i) + 0.7 * (h1 - 0.5)) / float(layer.count);
    float base = spread + seed + 0.06 * sin(c.s * 0.45 + seed);
    float2 dir = float2(cos(base), sin(base));

    // It grows out in the portal's wake, each at its own moment, and draws back in as the light
    // lifts away.
    float grown = saturate((c.since - u.run.z - 0.5 * h3) / (0.9 + 0.5 * h4));
    float growth = 1.0 - pow(1.0 - grown, 3.0);
    float lift = smoothstep(u.run.y + 0.35 * h4, u.run.x, c.s);
    float reach_px = edge_distance(dir, u) * (layer.reach + 0.25 * h2) * growth * (1.0 - 0.7 * lift * lift);
    if (r > reach_px + 40.0 || reach_px < 40.0) return float3(0.0);
    float along = r / reach_px;

    // The wave runs out along it, and a quicker one on top; the tip whips. They are sideways
    // offsets in points, so the bends are the same size whatever the radius, and `slope` is how much
    // the curve has turned from radial at `r`.
    float k = 2.0 * M_PI_F / (450.0 + 300.0 * h2);
    float omega = layer.speed * (0.8 + 0.5 * h4);
    float amp = layer.sway * (0.7 + 0.6 * h1);
    float whip = layer.curl * (0.6 + 0.8 * h2);
    float phase1 = k * r - omega * c.s + 6.2831853 * h3;
    float phase2 = 2.3 * k * r - 1.7 * omega * c.s + 6.2831853 * h4;
    float phase3 = 0.8 * omega * c.s + 6.2831853 * h3;
    float swell = pow(along, 1.2);
    float dswell = 1.2 * pow(along, 0.2) / reach_px;
    float wave = sin(phase1) + TENTACLE_OVERTONE * sin(phase2);
    float dwave = k * (cos(phase1) + TENTACLE_OVERTONE * 2.3 * cos(phase2));
    float side = amp * swell * wave + whip * along * along * along * sin(phase3);
    float dside = amp * (dswell * wave + swell * dwave) + whip * 3.0 * along * along / reach_px * sin(phase3);
    float theta = base + side / r;
    float tilt = dside - side / r;

    // How far this point is from the curve, across it: the angle between them over the sheer of
    // the curve there.
    float off = phi - theta;
    off -= 6.2831853 * round(off * 0.15915494);
    float across = r * off / sqrt(1.0 + tilt * tilt);

    // The ribbon: a sharp, bright edge on one side, as the hem of an aurora curtain, and on the
    // other rays that feather away, longer where they are brighter, sliding out along it.
    float width = 4.0 + layer.width * pow(1.0 - saturate(along), 0.8) * (0.45 + 0.55 * growth);
    float edge_side = across * (h1 > 0.5 ? 1.0 : -1.0);
    float rays = 0.65 * noise(float2(r * 0.055 - c.s * 1.2, seed + 10.0 * h1))
        + 0.35 * noise(float2(r * 0.16 - c.s * 0.8, seed + 20.0 * h2));
    // Mist (Voyage): gentler stripes, a wider feather, a soft edge instead of a line, and haze that fills
    // between them, so the arms are blurred and drift like mist.
    float mist = u.mist;
    rays = mix(rays, 0.5, 0.45 * mist);
    float feather = width * (0.3 + 1.1 * rays) * (1.0 + 0.9 * mist);
    bool sharp = edge_side >= 0.0;
    // Where the arms are big their lines are thick: the hot edge and the sharp side's flank scale up.
    float line = 1.0 + saturate(u.run.w - 1.0);
    float reach_across = sharp ? (0.16 * width + 1.5) * (1.0 + 0.5 * (line - 1.0)) * (1.0 + 2.2 * mist) : feather;
    float profile = exp(-pow(abs(edge_side) / reach_across, sharp ? mix(2.0, 1.3, mist) : mix(1.4, 1.0, mist)));
    float rim = exp(-pow(edge_side / ((0.05 * width + 1.5) * line * (1.0 + 2.8 * mist)), 2.0)) * (1.0 - 0.55 * mist);
    float haze = (0.05 + 0.20 * mist) * exp(-abs(across) / (width * (2.5 + 2.5 * mist)));
    // Bands of light rolling out along it.
    float waves = smoothstep(-0.3, 0.9, sin(r * 0.011 - c.s * 1.5 + 6.2831853 * h2)
                                         + 0.6 * sin(r * 0.026 - c.s * 2.2 + 6.2831853 * h3));
    // The body thins away into the head rather than ending on a line.
    float emerge = smoothstep(0.0, 0.1, grown) * smoothstep(0.0, 0.06, along)
        * (1.0 - smoothstep(0.55, 1.0, along));
    float fade = pow(1.0 - lift, 1.2);

    // Cyan to violet along it, and violet where the rays fray; hot at the edge.
    float3 colour = mix(layer.root, layer.mid, smoothstep(0.0, 0.5, along));
    colour = mix(colour, layer.tip, 0.8 * smoothstep(0.6, 1.2, along));
    colour = mix(colour, layer.tip, smoothstep(0.1, 1.0, -edge_side / width));
    colour = mix(colour, float3(1.0), 0.5 * rim);
    float body = (0.75 * profile + 0.9 * rim) * (0.25 + 0.75 * waves) + haze;
    return colour * body * emerge * layer.gain * fade;
}

// All the arms of `layer`.
static float3 tentacle_layer(float2 p, TentacleLayer layer, WormholeClock c, constant Uniforms& u) {
    float r = length(p);
    float phi = atan2(p.y, p.x);
    float3 light = float3(0.0);
    for (int i = 0; i < layer.count; i++) {
        light += tentacle(p, r, phi, i, layer, c, u);
    }
    return light;
}

// All the arms of both layers at `p`, behind the window's edge `wd`: what blooms once the portal has
// opened, as Tentacles and Voyage show it.
static float3 tentacle_light(float2 p, WormholeClock c, float wd, constant Uniforms& u) {
    float3 white = float3(1.0);
    // In the order of `TentacleLayer`'s fields: arms, width, reach, sway, curl, speed, seed, gain,
    // then the colours at the root, along the arm and at its head. The far arms are finer, longer,
    // slower and violet; the near ones thick and bright.
    // `u.run.w` is how big the arms are: wider and swinging further, not more of them.
    float big = u.run.w;
    TentacleLayer far_arms = {
        8, 40.0 * big, 1.15, 55.0 * big, 60.0 * big, 1.3, 2.0, 0.55,
        mix(u.c1.rgb, white, 0.35), mix(u.c1.rgb, u.c3.rgb, 0.4), u.c3.rgb,
    };
    TentacleLayer near_arms = {
        7, 56.0 * big, 1.05, 70.0 * big, 90.0 * big, 1.8, 5.0, 1.2,
        mix(u.c2.rgb, white, 0.5), u.c2.rgb, mix(u.c1.rgb, u.c3.rgb, 0.7),
    };
    float3 light = tentacle_layer(p, far_arms, c, u) + tentacle_layer(p, near_arms, c, u);
    // The same soft shoulder as the curtains, so where arms cross they glow rather than clip.
    return light / (1.0 + 0.35 * max3(light)) * smoothstep(0.0, 4.0, wd);
}

static float4 tentacles(float2 p, constant Uniforms& u) {
    Wormhole w = wormhole(p, u);
    if (w.clock.since > 0.0) {
        w.light += tentacle_light(p, w.clock, w.wd, u);
    }
    return wormhole_finish(w, p, u);
}
