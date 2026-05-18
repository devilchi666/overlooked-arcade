// oa-render — blur.wgsl
//
// Separable 5-tap Gaussian blur used by Phosphor preset (Phase 3 slice B).
// Run twice — once horizontally, once vertically — to approximate a 25-tap
// 2D Gaussian at 1/5th the sample count. The uniform's `direction_is_x`
// flag picks the axis; the same compiled pipeline is used for both passes,
// just with a different uniform value bound per pass.
//
// Kernel: 5-tap Gaussian, σ ~= 1.0 in source-pixel space. Weights are
// pre-normalized to sum to 1.0. The kernel size is intentionally small so
// the bloom stays subtle on small framebuffers (Lynx 160×102 would mush
// completely under a wider kernel).

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)        uv:       vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Standard oversized triangle covering the unit-square UV region. Note
    // the y is NOT flipped here — intermediate textures are sampled in the
    // same orientation we wrote them in, so no flip is needed. The final
    // blit (blit.wgsl) handles the texture-coords y-flip for the swapchain.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 2.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(positions[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}

struct Uniforms {
    /// 1 = sample along x axis (horizontal blur), 0 = sample along y axis
    /// (vertical blur).
    direction_is_x: u32,
    /// Source framebuffer width in pixels. Used to compute the per-pixel
    /// UV step (1.0 / fb_width) for the blur tap offsets.
    fb_width: u32,
    /// Source framebuffer height in pixels. Used for vertical blur step
    /// and stays in the uniform so we don't need a separate buffer per axis.
    fb_height: u32,
    _pad: u32,
};

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 5-tap Gaussian. Weights for σ=1: ≈ {0.0613, 0.2447, 0.3880, 0.2447, 0.0613}.
    let weights = array<f32, 5>(0.0613, 0.2447, 0.3880, 0.2447, 0.0613);
    let offsets = array<f32, 5>(-2.0, -1.0, 0.0, 1.0, 2.0);

    let step = select(
        vec2<f32>(0.0, 1.0 / max(f32(u.fb_height), 1.0)),
        vec2<f32>(1.0 / max(f32(u.fb_width), 1.0), 0.0),
        u.direction_is_x == 1u,
    );

    var acc = vec4<f32>(0.0);
    for (var i = 0u; i < 5u; i = i + 1u) {
        let uv = in.uv + step * offsets[i];
        acc = acc + textureSample(input_tex, input_sampler, uv) * weights[i];
    }
    return acc;
}
