fragment float4 intro_fragment(float4 pos [[position]], constant Uniforms& u [[buffer(0)]],
                               constant float2* trail [[buffer(1)]]) {
    float2 p = pos.xy / u.scale - u.center;
    switch (u.style) {
        case 0: return emerge(p, u);
        case 1: return vortex(p, u);
        case 2: return collapse(p, u);
        case 4: return ftl(p, u);
        case 5: return aurora(p, u);
        case 6: return tentacles(p, u);
        case 7: return voyage(p, u, trail);
        default: return glow(p, u);
    }
}
