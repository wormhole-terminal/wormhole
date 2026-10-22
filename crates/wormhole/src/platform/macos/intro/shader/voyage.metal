// --- Voyage --------------------------------------------------------------------------------------

// The wormhole run with a ship flown through it (`wormhole_core::intro::voyage`): the `>` of the
// app icon's `>_`, turned to point up as a `^`, big in front of the camera. It rises into the
// funnel, takes the jump, swoops across the window when the portal opens and docks on the logo, where
// the icon's own chevron is; the tentacles bloom from there. Core flies the ship (`u.ship`, `u.ship_fx`
// and the comet trail); this draws it, over everything else, window included.

// The hull's details were drawn for an icon whose chevron had a 97-unit stroke: its canvas-unit
// numbers scale with the stroke the icon has now (U0), and what is sized by the ship in points goes by
// how big its chevron shows (VIS, against the 445-unit-wide chevron of that icon).
constant float U0 = VOYAGE_STROKE / 97.0;
constant float VIS = (2.0 * VOYAGE_ARM_X + VOYAGE_STROKE) / 445.0;

// The ship's hull: the distance, in the icon's canvas units, from `l` (about the chevron's middle,
// the ship pointing up) to the chevron's round-capped stroke: two arms from the apex to either end of
// the tail. VOYAGE_*: core's `voyage` module keeps them the icon's.
static float sd_chevron(float2 l) {
    float2 q = float2(abs(l.x), l.y);
    float2 a = float2(0.0, VOYAGE_APEX_Y);
    float2 ba = float2(VOYAGE_ARM_X, VOYAGE_ARM_Y) - a;
    float2 pa = q - a;
    float h = saturate(dot(pa, ba) / dot(ba, ba));
    return length(pa - ba * h) - 0.5 * VOYAGE_STROKE;
}

// The same hull seen from behind as it flies off into the vortex: the nose is further away, so it is
// narrower and its stroke thinner than the tail's, which is nearest. `persp` 0 is the icon's flat
// chevron, which the ship has to be again when it docks.
static float sd_hull(float2 l, float persp) {
    float far = saturate((VOYAGE_ARM_Y - l.y) / (VOYAGE_ARM_Y - VOYAGE_APEX_Y));
    l.x /= 1.0 - 0.26 * persp * far;
    float thinning = 1.0 - 0.24 * persp * far;
    return sd_chevron(l) + 0.5 * VOYAGE_STROKE * (1.0 - thinning);
}

// The hull's distance with the two arms blended where they meet, for the shading only: the lit tube
// then turns smoothly round the apex instead of creasing there.
static float sd_hull_soft(float2 l, float persp) {
    float far = saturate((VOYAGE_ARM_Y - l.y) / (VOYAGE_ARM_Y - VOYAGE_APEX_Y));
    l.x /= 1.0 - 0.26 * persp * far;
    float thinning = 1.0 - 0.24 * persp * far;
    float2 a = float2(0.0, VOYAGE_APEX_Y);
    float2 ba = float2(VOYAGE_ARM_X, VOYAGE_ARM_Y) - a;
    float2 pl = l - a;
    float2 pr = float2(-l.x, l.y) - a;
    float dl = length(pl - ba * saturate(dot(pl, ba) / dot(ba, ba)));
    float dr = length(pr - ba * saturate(dot(pr, ba) / dot(ba, ba)));
    float blend = 44.0 * U0;
    float h = saturate(0.5 + 0.5 * (dr - dl) / blend);
    float d = mix(dr, dl, h) - blend * h * (1.0 - h);
    return d - 0.5 * VOYAGE_STROKE * thinning;
}

// An engine's plume at `lc` (canvas units, ship axes): a flame behind one tail end, narrowing and
// cooling from white-hot to the theme's blue and violet, flickering, a long one under thrust.
static float3 engine_plume(float2 lc, float thrust, constant Uniforms& u) {
    float dy = lc.y - VOYAGE_ARM_Y;
    float dx = abs(lc.x) - VOYAGE_ARM_X;
    float len = (200.0 + 600.0 * thrust) * U0;
    if (dy > len * 3.2) return float3(0.0);
    // Lit inside the hull's end and nowhere further up, with no line where it starts.
    float born = smoothstep(-30.0 * U0, 30.0 * U0, dy);
    float t = saturate(dy / len);
    dy = max(dy, 0.0);
    // A teardrop: it swells just behind the nozzle and narrows to nothing.
    float radius = 0.5 * VOYAGE_STROKE * (0.7 + 0.55 * smoothstep(0.0, 0.18, t) * (1.0 - smoothstep(0.18, 1.0, t))) + 4.0 * U0;
    float x = dx / radius;
    float flicker = 0.75 + 0.25 * noise(float2(dy / U0 * 0.010 - u.time * 34.0, 5.0));
    float pulse = 0.92 + 0.08 * sin(u.time * 47.0);
    // White-hot core, a cyan body, a violet halo, and bright shock diamonds in the core.
    float core = exp(-pow(x / 0.40, 2.0)) * exp(-dy / (0.24 * len));
    float body = exp(-pow(x / 0.95, 2.0)) * exp(-dy / (0.45 * len));
    float outer = exp(-pow(x / 1.8, 2.0)) * exp(-dy / (0.85 * len)) * 0.40;
    // A slow, smooth banding down the flame, not beads.
    float diamonds = 1.0 + 0.12 * sin(dy / U0 * 0.03 - u.time * 14.0) * (1.0 - t);
    float3 cyan = mix(u.c2.rgb, float3(1.0), 0.35);
    float3 violet = mix(u.c2.rgb, u.c3.rgb, 0.7);
    float3 colour = float3(1.0) * core * 1.5 * diamonds + cyan * body * 1.1 + violet * outer;
    return colour * born * flicker * pulse * (0.35 + 0.65 * thrust);
}

// The comet trail: where the ship has been, drawn through the points core gives, narrow at the ship and
// wide behind it, fading with age; nothing where it hasn't moved.
static float3 comet_trail(float2 sc, constant Uniforms& u, constant float2* trail) {
    float3 hot = mix(u.c2.rgb, float3(1.0), 0.7);
    float3 cool = mix(u.c2.rgb, u.c3.rgb, 0.5);
    float base = 3.0 + u.ship.z * VIS * 0.045;
    float3 light = float3(0.0);
    for (int i = 0; i < 23; i++) {
        float2 a = trail[i];
        float2 b = trail[i + 1];
        float2 pa = sc - a;
        float2 ba = b - a;
        float h = saturate(dot(pa, ba) / max(dot(ba, ba), 1e-3));
        float dist = length(pa - ba * h);
        float age = (float(i) + h) / 23.0;
        float moving = smoothstep(1.5, 9.0, length(ba));
        float sigma = base * (1.0 + 1.6 * age);
        float glow = exp(-pow(dist / sigma, 2.0)) * pow(1.0 - age, 1.5) * moving;
        light += mix(hot, cool, age) * glow;
    }
    return light * 1.1;
}

// The ship and what it throws off, premultiplied, for the point `sc` (points from the top left).
static float4 voyage_ship(float2 sc, constant Uniforms& u, constant float2* trail) {
    float thrust = u.ship_fx.x;
    float opacity = u.ship_fx.y;
    float flare = u.ship_fx.z;
    float bank = u.ship_fx.w;
    float3 ice = mix(u.c1.rgb, float3(1.0), 0.55);
    float3 hot = mix(u.c2.rgb, float3(1.0), 0.7);
    float3 light = float3(0.0);
    float3 body = float3(0.0);
    float cover = 0.0;
    float shadow = 0.0;

    if (opacity > 0.001) {
        float k = u.ship.z / VOYAGE_CANVAS;
        float2 d = sc - u.ship.xy;
        float ch = cos(u.ship.w);
        float sh = sin(u.ship.w);
        // The ship's own axes: x to its right, y towards its tail; narrower when it rolls onto a wing.
        float2 l = float2(dot(d, float2(ch, sh)), dot(d, float2(-sh, ch)));
        l.x /= max(cos(bank), 0.5);
        float2 lc = l / k;

        float zs = u.ship.z * VIS;
        if (length(d) < zs * 2.6) {
            // Seen from behind, flying into the vortex when big; the flat icon glyph once it is small.
            float persp = smoothstep(110.0, 340.0, u.ship.z);
            float sd = sd_hull(lc, persp) * k;
            cover = smoothstep(0.8, -0.8, sd);
            // As the icon's glyph: white at the nose to pale blue at the tail. Large, it is a rounded
            // tube lit from the upper left (a normal from the distance's slope), with a highlight along
            // each arm and a cool shadow side; small, it is the flat glyph, as the icon has it.
            float along = saturate((lc.y - (VOYAGE_APEX_Y - 0.5 * VOYAGE_STROKE)) / ((VOYAGE_ARM_Y - VOYAGE_APEX_Y) + VOYAGE_STROKE));
            float3 hull = mix(float3(1.0), mix(float3(0.75, 0.9, 1.0), ice, 0.3), along);
            // A normal from the blended hull's slope: a rounded tube, 0 across its axis to 1 at the rim.
            float soft = sd_hull_soft(lc, persp);
            float round_ = saturate(1.0 + soft / (0.5 * VOYAGE_STROKE));
            float e = 2.0 * U0;
            float2 slope = float2(sd_hull_soft(lc + float2(e, 0.0), persp) - sd_hull_soft(lc - float2(e, 0.0), persp),
                                  sd_hull_soft(lc + float2(0.0, e), persp) - sd_hull_soft(lc - float2(0.0, e), persp));
            float3 normal = float3(normalize(slope + 1e-5) * round_, sqrt(max(1.0 - round_ * round_, 0.0)));
            float2 sun_screen = normalize(float2(-0.45, -0.75));
            float2 sun_local = float2(dot(sun_screen, float2(ch, sh)), dot(sun_screen, float2(-sh, ch)));
            float3 sun = normalize(float3(sun_local, 0.45));
            // Soft light: wrapped round the tube so the shadow side turns gently rather than cutting off,
            // a deep cool shadow with the theme's blue in it, a pale rim from the vortex behind, a dimming
            // where the arms meet, and the engines lighting the tail ends from underneath.
            float lit = saturate((dot(normal, sun) + 0.3) / 1.3);
            lit = lit * lit * (3.0 - 2.0 * lit);
            float3 shadow_col = mix(float3(0.06, 0.09, 0.24), u.c1.rgb * 0.30, 0.5);
            float3 mid_col = mix(float3(0.42, 0.58, 0.95), ice, 0.35);
            float3 lit_col = mix(float3(0.86, 0.94, 1.0), hull, 0.55);
            float3 shaded = mix(shadow_col, mix(mid_col, lit_col, smoothstep(0.45, 0.95, lit)), smoothstep(0.05, 0.6, lit));
            float rim_light = pow(1.0 - normal.z, 2.4);
            shaded += mix(u.c2.rgb, float3(1.0), 0.45) * rim_light * (0.12 + 0.3 * (1.0 - lit));
            shaded *= 1.0 - 0.38 * exp(-length(lc - float2(0.0, VOYAGE_APEX_Y + 85.0 * U0)) / (75.0 * U0));
            float under = exp(-length(float2(abs(lc.x) - VOYAGE_ARM_X, lc.y - VOYAGE_ARM_Y)) / (75.0 * U0)) * (0.25 + 0.75 * thrust);
            shaded += mix(hot, u.c2.rgb, 0.4) * under * 0.4;
            float spec = pow(saturate(dot(reflect(-sun, normal), float3(0.0, 0.0, 1.0))), 28.0);
            shaded += spec * 0.6;
            body = mix(hull, shaded, persp * 0.9);
            // A soft shadow behind it, down and to the right of the light, so it stands off the stars.
            float2 offset_screen = float2(0.025, 0.045) * zs;
            float2 offset_local = float2(dot(offset_screen, float2(ch, sh)), dot(offset_screen, float2(-sh, ch))) / k;
            float shadow_sd = sd_hull(lc - offset_local, persp) * k;
            shadow = exp(-max(shadow_sd, 0.0) / (0.05 * zs)) * 0.42 * (1.0 - cover);
            // A halo, brighter as the engines burn.
            float halo = exp(-max(sd, 0.0) / (0.06 * zs)) * (0.10 + 0.22 * thrust);
            light += mix(u.c1.rgb, u.c2.rgb, 0.5) * halo * (1.0 - cover) * opacity;
            // The engines: a hot disc in a ring at each tail end, and the plumes.
            float2 nz = float2(abs(lc.x) - VOYAGE_ARM_X, (lc.y - (VOYAGE_ARM_Y + 14.0 * U0)) * 1.5);
            float nd = length(nz);
            float3 cyan = mix(u.c2.rgb, float3(1.0), 0.4);
            float3 nozzle_light = (float3(1.0) * smoothstep(34.0 * U0, 10.0 * U0, nd) * 1.3
                                   + cyan * exp(-pow((nd - 34.0 * U0) / (7.0 * U0), 2.0)) * 0.8
                                   + cyan * exp(-nd / (60.0 * U0)) * 0.25) * (0.3 + 0.7 * thrust);
            light += nozzle_light * (1.0 - 0.5 * cover) * opacity;
            light += engine_plume(lc, thrust, u) * opacity;
            // A shockwave ring thrown off at the jump.
            float since_jump = u.time - WORMHOLE_JUMP;
            if (since_jump > 0.0 && since_jump < 0.8) {
                float radius = zs * 1.1 * (1.0 - pow(1.0 - since_jump / 0.8, 3.0));
                light += hot * exp(-pow((length(l) - radius) / (4.0 + 10.0 * since_jump), 2.0)) * pow(1.0 - since_jump / 0.8, 1.6) * 0.55;
            }
            // The afterburners' flash, with a long streak across the lens.
            float burst = exp(-length(l) / (0.55 * zs)) * 0.9
                + exp(-abs(l.y) / (0.025 * zs)) * exp(-abs(l.x) / (1.1 * zs)) * 1.2;
            light += hot * burst * flare;
        }
    }
    // The comet trail, through the swoop.
    if (u.time > WORMHOLE_EXIT && u.time < VOYAGE_LAND + 0.7) {
        light += comet_trail(sc, u, trail);
    }

    float3 rgb = body * cover * opacity + light;
    float alpha = clamp(max(max(cover * opacity, max3(light)), shadow * opacity), 0.0, 1.0);
    return float4(min(rgb, float3(alpha)), alpha);
}

static float4 voyage(float2 p, constant Uniforms& u, constant float2* trail) {
    Wormhole w = wormhole(p, u);
    if (VOYAGE_AURORA > 0.5 && w.clock.since > 0.0) {
        // The arms bloom behind the window once the ship has docked (`u.run.z`), when core has them on.
        w.light += tentacle_light(p, w.clock, w.wd, u);
    }
    float4 base = wormhole_finish(w, p, u);
    float4 ship = voyage_ship(p + u.center, u, trail);
    // The ship over everything, the window too: it flies over the page to the logo.
    return float4(ship.rgb + base.rgb * (1.0 - ship.a), ship.a + base.a * (1.0 - ship.a));
}
