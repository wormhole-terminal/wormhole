// --- Glow (Reduce Motion) ------------------------------------------------------------------------

// The edges glow and fade; nothing moves and the screen stays as it is.
static float4 glow(float2 p, constant Uniforms& u) {
    float t = u.t;
    float d = sd_round_rect(p, u.half_size, u.radius);
    float env = smoothstep(0.0, 0.2, t) * (1.0 - smoothstep(0.45, 1.0, t));
    float3 hot = mix(u.c1.rgb, float3(1.0), 0.6);
    float3 e = u.c1.rgb * exp(-max(d, 0.0) / 24.0) * step(0.0, d) * 0.8 + hot * exp(-abs(d) / 1.5) * 0.7;
    return compose(float3(0.0), 0.0, e * env);
}
