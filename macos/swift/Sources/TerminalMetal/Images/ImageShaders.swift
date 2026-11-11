// The picture pipeline: one textured quad per draw. The texture holds straight alpha as decoded; the fragment
// premultiplies, since the pass blends premultiplied. Buffers 3 and 4 so nothing the cell pipelines set at
// indices 0 to 2 is disturbed.

enum ImageShaders {
    static let source = """
        #include <metal_stdlib>
        using namespace metal;

        struct ImageQuad { float x, y, width, height; float u0, v0, u1, v1; };

        struct ImageOut {
            float4 position [[position]];
            float2 uv;
        };

        vertex ImageOut image_vertex(
            uint vid [[vertex_id]], constant ImageQuad& q [[buffer(3)]], constant float2& viewport [[buffer(4)]])
        {
            float2 corner = float2(float(vid & 1u), float(vid >> 1));
            float2 px = float2(q.x, q.y) + corner * float2(q.width, q.height);
            ImageOut out;
            out.position = float4(px.x / viewport.x * 2.0 - 1.0, 1.0 - px.y / viewport.y * 2.0, 0.0, 1.0);
            out.uv = mix(float2(q.u0, q.v0), float2(q.u1, q.v1), corner);
            return out;
        }

        constexpr sampler pictureSampler(filter::linear, mip_filter::linear, address::clamp_to_edge);

        fragment float4 image_fragment(ImageOut in [[stage_in]], texture2d<float> picture [[texture(0)]]) {
            float4 c = picture.sample(pictureSampler, in.uv);
            return float4(c.rgb * c.a, c.a);
        }
        """
}
