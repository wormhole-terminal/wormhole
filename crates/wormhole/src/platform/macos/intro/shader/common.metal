// The wormhole intro (wormhole_core::onboarding::intro): one full-screen triangle over an overlay that covers
// the whole screen, and a fragment shader per style, each in its own file. The screen sinks into
// deep space, a procedural starfield bends around the wormhole the way gravitational lensing bends
// it, and space lets go of the window again. Everything is in points, centred on the window; the
// output is premultiplied, and wherever the window is, the overlay stays see-through so the window
// shows.
//
// Compiled at run time by `platform::macos::intro`, which joins the files in the order each needs the one
// before (Metal has no #include for a library built from source) behind the moments core owns
// (`wormhole_core::onboarding::intro::shader_constants`: FTL_* and WORMHOLE_*).

#include <metal_stdlib>
using namespace metal;

// Mirrors `Uniforms` in metal.rs, field for field.
struct Uniforms {
    float2 size;       // the overlay (the screen), points
    float2 center;     // the window's centre in the overlay, points from the top left
    float2 half_size;  // the window's half size at rest, points
    float scale;       // pixels per point in the drawable
    float t;           // progress, 0 to 1
    float time;        // seconds since the start
    float win_scale;   // the window's own scale right now
    float radius;      // the window's corner radius, points
    float win_alpha;   // the window's own opacity right now
    uint style;        // Style::shader_id
    float speed;       // FTL: how fast space streams past, points a second
    float travel;      // FTL: how far it has streamed, points
    float mist;        // Voyage: 1 for the smooth plasma tunnel and the arms blurred into mist
    float4 run;        // the wormhole styles: the run's seconds, when the light lifts, the bloom's delay
    float4 ship;       // Voyage: the ship's chevron, points from the top left: x, y, its icon box's side, heading
    float4 ship_fx;    // Voyage: thrust, opacity, afterburner flare, bank
    float4 c1;         // the theme's primary colour
    float4 c2;         // the theme's info colour
    float4 c3;         // the theme's terminal magenta: FTL's violet
};

vertex float4 intro_vertex(uint vid [[vertex_id]]) {
    float2 p = float2(float((vid << 1) & 2), float(vid & 2));
    return float4(p * 2.0 - 1.0, 0.0, 1.0);
}

// --- noise ---------------------------------------------------------------------------------------

static float hash21(float2 p) {
    p = fract(p * float2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

static float noise(float2 p) {
    float2 i = floor(p);
    float2 f = fract(p);
    float2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i), hash21(i + float2(1, 0)), u.x),
               mix(hash21(i + float2(0, 1)), hash21(i + float2(1, 1)), u.x), u.y);
}

static float fbm(float2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        v += a * noise(p);
        p = p * 2.03 + float2(1.7, 9.2);
        a *= 0.5;
    }
    return v;
}

static float2 rotate(float2 p, float angle) {
    float c = cos(angle);
    float s = sin(angle);
    return float2(c * p.x - s * p.y, s * p.x + c * p.y);
}

static float max3(float3 v) { return max(v.x, max(v.y, v.z)); }

// --- shapes --------------------------------------------------------------------------------------

static float sd_round_rect(float2 p, float2 b, float r) {
    float2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

// The throat: a circle of radius `circle` morphing (`m`) into the window's rounded rectangle.
static float sd_throat(float2 p, float circle, float2 hs, float r, float m) {
    return mix(length(p) - circle, sd_round_rect(p, hs, r), m);
}

static float2 throat_normal(float2 p, float circle, float2 hs, float r, float m) {
    const float e = 1.5;
    float dx = sd_throat(p + float2(e, 0), circle, hs, r, m) - sd_throat(p - float2(e, 0), circle, hs, r, m);
    float dy = sd_throat(p + float2(0, e), circle, hs, r, m) - sd_throat(p - float2(0, e), circle, hs, r, m);
    float2 g = float2(dx, dy);
    float l = length(g);
    return l > 1e-4 ? g / l : float2(0.0);
}

// --- the sky -------------------------------------------------------------------------------------

// Stars: three layers of points with a few bright ones, tinted from the theme's colours towards
// white the way real stars sit near white. `seed` gives the far side of the wormhole its own sky.
static float3 stars(float2 q, float seed, constant Uniforms& u) {
    float3 col = float3(0.0);
    float3 warm = mix(u.c2.rgb, float3(1.0), 0.75);
    float3 cool = mix(u.c1.rgb, float3(1.0), 0.65);
    for (int i = 0; i < 3; i++) {
        float density = 0.05 - 0.012 * float(i);
        float2 g = q * density + float2(17.3, 9.1) * (float(i) + seed);
        float2 cell = floor(g);
        float2 f = fract(g) - 0.5;
        float h = hash21(cell + 31.0 * float(i) + seed);
        if (h > 0.22) continue;
        float2 o = (float2(hash21(cell + 1.3), hash21(cell + 2.7)) - 0.5) * 0.7;
        float d = length(f - o) / density;
        float size = 0.55 + 0.9 * hash21(cell + 5.1) + 0.5 * float(i);
        float bright = 0.35 + 2.4 * pow(hash21(cell + 8.8), 8.0);
        col += mix(warm, cool, hash21(cell + 4.4)) * exp(-d * d / (size * size)) * bright;
    }
    return col;
}

// Gas: two layers of noise in the theme's colours, faint, as a sky shows it.
static float3 nebula(float2 q, float seed, float strength, constant Uniforms& u) {
    float a = fbm(q * 0.0022 + seed);
    float b = fbm(q * 0.0045 - seed * 1.7 + 3.0);
    float dust = fbm(q * 0.009 + seed * 0.3);
    float3 gas = u.c1.rgb * pow(a, 3.2) * 0.9 + u.c2.rgb * pow(b, 3.6) * 0.7;
    return gas * strength * (0.55 + 0.6 * dust);
}

static float3 sky(float2 q, float seed, float gas, constant Uniforms& u) {
    return stars(q, seed, u) + nebula(q, seed, gas, u);
}

// --- composing -----------------------------------------------------------------------------------

// Space `space` covering the screen by `cover`, plus light `e`; premultiplied.
static float4 compose(float3 space, float cover, float3 e) {
    float3 rgb = max(space * cover + e, 0.0);
    float a = clamp(max(cover, max3(rgb)), 0.0, 1.0);
    return float4(min(rgb, float3(a)), a);
}

// The overlay over the window (`wd` its distance): see-through as far as the window has faded in.
static float4 over_window(float4 c, float wd, constant Uniforms& u) {
    float inside = smoothstep(0.75, -0.75, wd);
    return c * mix(1.0, 1.0 - u.win_alpha, inside);
}
