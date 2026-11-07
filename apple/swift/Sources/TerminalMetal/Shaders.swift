// The shaders, compiled from source at start-up (`makeLibrary(source:)`, as the app's intro does): no
// Metal toolchain is needed at build time. Two instanced quad pipelines share one vertex scheme: a unit
// quad as a four-vertex strip, placed in pixels by the instance. Colours arrive unresolved (tag << 24 |
// value) and are resolved here against the palette, so a theme change rebuilds no instance.

enum Shaders {
    static let source = """
        #include <metal_stdlib>
        using namespace metal;

        struct Uniforms {
            float2 viewport;   // pixels
            float dim;         // the factor SGR 2 multiplies a colour by
            float blink;       // 1, or the factor blinking text is faded by in the off half
        };

        struct SolidInstance { float x, y, width, height; uint color; uint style; };
        struct GlyphInstance { float x, y, width, height; ushort ax, ay, aw, ah; uint color; uint flags; };

        // tag 2 is truecolor in the low 24 bits; anything else indexes the palette (named slots and indexed
        // colours share it).
        static float3 resolve(uint packed, constant float4* palette) {
            if ((packed >> 24) == 2u) {
                return float3(float((packed >> 16) & 0xFFu), float((packed >> 8) & 0xFFu), float(packed & 0xFFu)) / 255.0;
            }
            return palette[packed & 0xFFFFu].rgb;
        }

        static float4 place(float x, float y, float w, float h, uint vid, constant Uniforms& u, thread float2& corner) {
            corner = float2(float(vid & 1u), float(vid >> 1));
            float2 px = float2(x, y) + corner * float2(w, h);
            return float4(px.x / u.viewport.x * 2.0 - 1.0, 1.0 - px.y / u.viewport.y * 2.0, 0.0, 1.0);
        }

        // MARK: solid quads (backgrounds, underlines, the cursor)

        struct SolidOut {
            float4 position [[position]];
            float3 color [[flat]];
            float2 local;               // pixels from the quad's top-left; x is absolute so patterns line up
            float2 size [[flat]];
            uint pattern [[flat]];
        };

        vertex SolidOut solid_vertex(
            uint vid [[vertex_id]], uint iid [[instance_id]],
            constant SolidInstance* instances [[buffer(0)]], constant Uniforms& u [[buffer(1)]],
            constant float4* palette [[buffer(2)]])
        {
            SolidInstance q = instances[iid];
            float2 corner;
            SolidOut out;
            out.position = place(q.x, q.y, q.width, q.height, vid, u, corner);
            out.color = resolve(q.color, palette) * (((q.style >> 8) & 1u) != 0u ? u.dim : 1.0);
            out.local = float2(q.x + corner.x * q.width, corner.y * q.height);
            out.size = float2(q.width, q.height);
            out.pattern = q.style & 0xFu;
            return out;
        }

        fragment float4 solid_fragment(SolidOut in [[stage_in]]) {
            float coverage = 1.0;
            float t = in.size.y;                       // the line's thickness (a curl's is a third of its height)
            if (in.pattern == 1u) {                    // dotted: dots one thickness wide, one apart
                coverage = fmod(in.local.x, t * 2.0) < t ? 1.0 : 0.0;
            } else if (in.pattern == 2u) {             // dashed: three thicknesses on, two off
                coverage = fmod(in.local.x, t * 5.0) < t * 3.0 ? 1.0 : 0.0;
            } else if (in.pattern == 3u) {             // curly: distance to a sine wave through the quad's middle
                float line = t / 3.0;
                float period = line * 6.0;
                float y = in.size.y * 0.5 + sin(in.local.x / period * 6.2831853) * line;
                coverage = 1.0 - smoothstep(line * 0.5, line * 0.5 + 1.0, abs(in.local.y - y));
            }
            return float4(in.color * coverage, coverage);
        }

        // MARK: glyph quads

        struct GlyphOut {
            float4 position [[position]];
            float3 color [[flat]];
            float2 uv;                  // pixels in the atlas texture
            float fade [[flat]];        // blinking text in the off half
            uint flags [[flat]];
        };

        vertex GlyphOut glyph_vertex(
            uint vid [[vertex_id]], uint iid [[instance_id]],
            constant GlyphInstance* instances [[buffer(0)]], constant Uniforms& u [[buffer(1)]],
            constant float4* palette [[buffer(2)]])
        {
            GlyphInstance g = instances[iid];
            float2 corner;
            GlyphOut out;
            out.position = place(g.x, g.y, g.width, g.height, vid, u, corner);
            out.color = resolve(g.color, palette) * ((g.flags & 1u) != 0u ? u.dim : 1.0);
            out.uv = float2(float(g.ax), float(g.ay)) + corner * float2(float(g.aw), float(g.ah));
            out.fade = (g.flags & 4u) != 0u ? u.blink : 1.0;
            out.flags = g.flags;
            return out;
        }

        constexpr sampler atlasSampler(coord::pixel, filter::nearest, address::clamp_to_edge);

        fragment float4 glyph_fragment(
            GlyphOut in [[stage_in]], texture2d<float> mask [[texture(0)]], texture2d<float> color [[texture(1)]])
        {
            if ((in.flags & 2u) != 0u) {
                return color.sample(atlasSampler, in.uv) * in.fade;   // premultiplied, as rasterised
            }
            float a = mask.sample(atlasSampler, in.uv).r * in.fade;
            return float4(in.color * a, a);
        }
        """
}
