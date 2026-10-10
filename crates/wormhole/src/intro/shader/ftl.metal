// --- FTL Jump ------------------------------------------------------------------------------------

// The jump's moments, FTL_JUMP and FTL_EXIT, are fractions of its run (core's shader_constants).

// Destiny's FTL, seen from aboard and heading into the screen: a charge of mist gathers at the
// screen's edges, a flash, and the stars stretch into streaks flying out from the centre through
// a tunnel of blue-violet plasma with a glow ahead. At the drop out the window arrives: a flash
// where it is, and a cloud that bursts round it and thins away.
static float4 ftl(float2 p, constant Uniforms& u) {
    float t = u.t;
    float2 hs = u.half_size * u.win_scale;
    float wd = sd_round_rect(p, hs, u.radius);
    // From the screen's centre, where the jump heads.
    float2 q = p - (u.size * 0.5 - u.center);
    float r = length(q);
    float ang = atan2(q.y, q.x);
    float far = length(u.size) * 0.5;
    float rn = r / far;

    float cover = smoothstep(0.0, 0.1, t) * (1.0 - smoothstep(0.84, 1.0, t)) * 0.97;
    float pre = smoothstep(0.04, FTL_JUMP, t) * (1.0 - smoothstep(FTL_JUMP - 0.02, FTL_JUMP + 0.02, t));
    float in_ftl = smoothstep(FTL_JUMP - 0.02, FTL_JUMP + 0.04, t)
        * (1.0 - smoothstep(FTL_EXIT - 0.01, FTL_EXIT + 0.025, t));
    float out_t = smoothstep(FTL_EXIT, 1.0, t);
    float after = smoothstep(FTL_EXIT - 0.01, FTL_EXIT + 0.03, t);
    float jump_flash = exp(-pow((t - FTL_JUMP) / 0.014, 2.0));
    float exit_flash = exp(-pow((t - FTL_EXIT) / 0.012, 2.0));

    float speed = u.speed / 5200.0;
    float zoom = u.travel * 0.0016;
    float3 violet_a = mix(u.c1.rgb, u.c3.rgb, 0.35);
    float3 hot = mix(u.c1.rgb, float3(1.0), 0.5);

    // Still stars either side of the jump; streaks during it.
    float still = 1.0 - in_ftl;
    float3 space = sky(q * (1.0 + 0.15 * after), 4.0, 0.6, u) * still;
    space += warp_streaks(q, zoom, speed, u) * in_ftl;

    // The tunnel: mist flowing outward past the screen's edges, streaked along its way.
    float4 mist = tunnel_mist(r, ang, rn, zoom, u);
    float3 tunnel = mist.rgb;
    float strands = mist.a;
    // The glow ahead, at the centre, breathing a little.
    float ahead = exp(-r / (70.0 + 25.0 * sin(u.time * 3.0))) + 0.35 * exp(-r / 260.0);

    float shimmer = 0.8 + 0.2 * sin(u.time * 23.0 + strands * 12.0);
    float3 e = tunnel * (pre * 0.6 * shimmer * smoothstep(0.35, 1.0, rn) + in_ftl);
    e += hot * ahead * (in_ftl * 0.55 + pre * 0.15);
    e += hot * jump_flash * (exp(-r / 300.0) * 1.2 + 0.35);

    // Arrival: a flash where the window is, and a cloud that bursts round it and thins.
    float cloud = fbm(p * 0.0035 + float2(out_t * 0.5, 1.0)) * fbm(p * 0.009 - out_t * 0.3);
    float reach = 120.0 + 900.0 * out_t;
    float burst = exp(-max(wd, 0.0) / reach) * after * (1.0 - out_t) * (1.0 - out_t);
    e += violet_a * cloud * 2.4 * burst;
    e += hot * exit_flash * (exp(-max(wd, 0.0) / 240.0) + 0.2);
    e += hot * exp(-abs(wd) / 1.4) * after * (1.0 - out_t) * 0.8;

    float4 c = compose(space, cover, e);
    // The window: hidden until it arrives, then veiled only by the cloud as it clears.
    float inside = smoothstep(0.75, -0.75, wd);
    float veil = max(1.0 - u.win_alpha, clamp(0.6 * cloud * burst + 0.5 * exit_flash, 0.0, 0.6));
    return mix(c, c * veil, inside);
}
