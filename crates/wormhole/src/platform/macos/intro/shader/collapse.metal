// --- Collapse ------------------------------------------------------------------------------------

// The screen fills with a turning spiral of stars and gas that falls inward onto the window,
// faster as it nears; its last light becomes the window's edge, and space lets go.
static float4 collapse(float2 p, constant Uniforms& u) {
    float t = u.t;
    float s = u.win_scale;
    float2 hs = u.half_size * s;
    float d = sd_round_rect(p, hs, u.radius * s);
    float rad = length(p);
    float ang = atan2(p.y, p.x);

    float cover = smoothstep(0.0, 0.18, t) * (1.0 - smoothstep(0.72, 0.95, t)) * 0.96;
    float pull = smoothstep(0.05, 0.75, t);
    float spin = 0.4 + 4.0 * pull;

    // Everything turns faster near the window and is drawn in as the collapse goes on: sample the
    // sky further out the further it has fallen.
    float size = min(hs.x, hs.y);
    float fall = 1.0 + 2.2 * pull * pull;
    float twist = spin * exp(-max(d, 0.0) / (size * 1.4)) + u.time * 0.35 * pull;
    float2 n = normalize(p + 1e-4);
    float einstein = size * 0.8 * cover;
    // Fading out towards the centre, where a point lens would pinch the sky to a point.
    float2 source = p - n * (einstein * einstein) / (max(d, 0.0) + size) * smoothstep(0.0, size, rad);
    source = rotate(source * fall, twist);
    float3 space = sky(source, 3.0, 0.8, u);

    // The spiral: log-spiral arms of gas, falling in.
    float lr = log(max(rad, 1.0));
    float dust = fbm(rotate(p, twist) * 0.008 + pull * 2.0);
    float arms = pow(0.5 + 0.5 * sin(2.0 * (ang + twist) - lr * 5.0 + u.time * 2.0 + dust * 3.0), 2.5);
    float front = max(length(u.size), 1.0) * (1.0 - pull) + 6.0;
    // Over the window's rectangle too, until the window fades in over it.
    float inside_front = smoothstep(front, front - 200.0, d);
    float3 gas = mix(u.c2.rgb, u.c1.rgb, arms) * arms * (0.35 + 0.9 * dust) * inside_front;
    float glow = exp(-max(d, 0.0) / (size * 0.6)) * inside_front;
    float3 e = gas * (0.25 + 1.3 * glow) * cover;

    // The window's edge: a flash as the front arrives, then nothing.
    float3 hot = mix(u.c1.rgb, float3(1.0), 0.65);
    float settle = smoothstep(0.62, 0.78, t) * (1.0 - smoothstep(0.8, 1.0, t));
    e += hot * (exp(-max(d, 0.0) / 14.0) * step(0.0, d) * 0.8 + exp(-abs(d) / 1.4) * 1.2) * settle;

    return over_window(compose(space, cover, e), d, u);
}
