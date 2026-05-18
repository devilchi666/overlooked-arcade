// oa-render — bezel.wgsl
//
// Phase 3 slice B-2 — Bezel overlay. Draws an RGBA texture stretched to
// fill the full surface (NOT the game viewport), alpha-blended over
// whatever the final blit drew. The bezel image is expected to have its
// own transparent center where the game shows through, and opaque outer
// regions that form the "TV frame" or whatever surrounding artwork the
// user has chosen. The blend state in the Rust pipeline does the alpha
// composite — this shader just samples and outputs `texture.rgba`
// unmodified; SrcAlpha * src + InvSrcAlpha * dst happens in fixed-function.
//
// "Respects scaling mode" (per ROADMAP) means: the bezel is drawn AFTER
// the scaled game blit, so the game's scaling mode + viewport rect are
// preserved. The bezel itself covers the whole window — users design
// their bezel image to match their preferred window dimensions.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)        uv:       vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    // Texture origin is top-left; NDC y goes up. Flip y so the bezel
    // image displays right-side-up.
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(positions[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}

@group(0) @binding(0) var bezel_tex: texture_2d<f32>;
@group(0) @binding(1) var bezel_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(bezel_tex, bezel_sampler, in.uv);
}
