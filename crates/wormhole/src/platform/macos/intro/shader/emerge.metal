// --- Emerge --------------------------------------------------------------------------------------

// The screen sinks into space; a wormhole opens where the window will be, bending the stars
// around it, with another sky inside. The window comes through the throat, which settles onto
// its edges, and space lets go.
static float4 emerge(float2 p, constant Uniforms& u) {
    float t = u.t;
    float s = u.win_scale;
    float2 hs = u.half_size * s;
    float r = u.radius * s;
    float ang = atan2(p.y, p.x);

    float cover = smoothstep(0.0, 0.16, t) * (1.0 - smoothstep(0.76, 1.0, t)) * 0.96;
    float open = smoothstep(0.1, 0.4, t);
    // Always wide enough to hold the window coming through it.
    float circle = max(open * 0.42 * min(u.half_size.x, u.half_size.y), length(hs) + 6.0);
    float m = smoothstep(0.42, 0.74, t);
    float d = sd_throat(p, circle, hs, r, m);
    float2 n = throat_normal(p, circle, hs, r, m);

    // Lensing: a point outside the throat sees the sky from nearer the throat (r - E²/r), so
    // stars stretch into arcs around it; space also turns with the spin (frame dragging).
    float size = mix(circle, min(hs.x, hs.y), m);
    float strength = open * (1.0 - smoothstep(0.72, 0.96, t));
    float einstein = size * 0.95 * strength;
    float reach = max(d, 0.0) + size;
    float2 source = p - n * (einstein * einstein) / reach;
    float drag = 2.6 * strength * exp(-max(d, 0.0) / (size * 1.2 + 40.0));
    source = rotate(source, drag + u.time * 0.12 * strength);
    float2 drift = float2(u.time * 9.0, u.time * 3.0);
    float3 space = sky(source + drift, 0.0, 1.0, u);

    // The photon ring: thin, hot, brighter on the side spinning towards you.
    float3 hot = mix(u.c1.rgb, float3(1.0), 0.6);
    float beam = 0.55 + 0.45 * sin(ang - u.time * 2.2);
    float ring_fade = open * (1.0 - smoothstep(0.82, 1.0, t));
    float3 ring = float3(exp(-abs(d - 1.2) / 1.4), exp(-abs(d) / 1.4), exp(-abs(d + 1.2) / 1.4));
    float3 e = hot * ring * (0.6 + 1.2 * beam) * ring_fade;
    // Hot gas swirling just outside the throat.
    float swirl = fbm(rotate(p, drag * 1.6 - u.time * 1.8) * 0.02);
    float halo = exp(-max(d, 0.0) / (18.0 + 0.25 * size)) * step(0.0, d);
    e += mix(u.c2.rgb, u.c1.rgb, swirl) * halo * (0.25 + 1.1 * swirl * beam) * ring_fade;

    // The far side: another sky, seen through the throat, magnified and turning.
    float2 far_q = rotate(p, -u.time * 0.25) * 0.55 - drift * 0.5;
    float3 far = sky(far_q, 7.0, 2.2, u) + u.c1.rgb * exp(d / (size * 0.35 + 1.0)) * 0.35;
    float throat = smoothstep(0.75, -0.75, d) * open;
    float4 c = mix(compose(space, cover, e), float4(far, 1.0), throat);

    float wd = sd_round_rect(p, hs, r);
    return over_window(c, wd, u);
}
