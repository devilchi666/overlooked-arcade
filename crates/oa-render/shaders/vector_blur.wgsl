// oa-render — vector_blur.wgsl
//
// First-pass shader for the VectorPhosphor preset chain. Sampled at
// every fragment of the H-blur pass (direction_is_x = 1) and the
// V-blur pass (direction_is_x = 0). The H pass also applies a
// bright-pass filter as it samples — only luminance above the
// `glow_threshold` uniform contributes. The V pass blurs the already-
// bright-passed result without re-thresholding.
//
// Kernel: 9-tap Gaussian, σ ≈ 2.5 in source-pixel space (about 2.5×
// the kernel width of the Phosphor preset's 5-tap σ=1.0). The wider
// kernel is what gives the Vectrex its signature halo around stroke
// tips. Weights pre-normalized to sum to 1.0.
//
// Bright-pass formula:
//   lum = dot(sample.rgb, vec3(0.299, 0.587, 0.114))
//   contribution = sample.rgb * smoothstep(threshold, threshold + 0.1, lum)
// The smoothstep band keeps a small soft transition at the threshold
// so almost-bright pixels still contribute partially. Dark background
// pixels return ~zero and don't fight the source-detail in the final
// composite.

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
    /// 1 = sample along x (H pass + bright-pass), 0 = sample along y
    /// (V pass, no bright-pass — input is already filtered).
    direction_is_x: u32,
    /// Source framebuffer width in pixels — drives the per-pixel UV
    /// step for the H blur.
    fb_width: u32,
    /// Source framebuffer height — drives V blur step.
    fb_height: u32,
    /// Luminance threshold for the bright-pass filter on the H pass.
    /// Vectrex backgrounds are pure black so a value of 0.5 cleanly
    /// separates strokes (luminance > 0.7) from any vector-rendering
    /// rasterization artifacts (< 0.2). The V pass ignores this.
    glow_threshold: f32,
};

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;

/// Standard luminance recipe (BT.601). Cheap; the kernel does this
/// 9× per fragment on the H pass which is still negligible vs the
/// fill rate of a 480×320-class framebuffer.
fn luminance(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}

/// Bright-pass — return rgb scaled by a smoothstep over the threshold.
/// A 0.1-wide soft band at the threshold keeps mid-bright pixels
/// contributing partially rather than abruptly cutting off.
fn bright_pass(rgb: vec3<f32>, threshold: f32) -> vec3<f32> {
    let lum = luminance(rgb);
    return rgb * smoothstep(threshold, threshold + 0.1, lum);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 9-tap Gaussian, σ ≈ 2.5. Weights from a normalized Gaussian PDF:
    //   exp(-x²/(2σ²)) / Σ
    // Pre-normalized so Σ weight = 1.0; offsets in source-pixel units.
    let weights = array<f32, 9>(
        0.0252, 0.0556, 0.1011, 0.1494, 0.1772,
        0.1494, 0.1011, 0.0556, 0.0252,
    );
    let offsets = array<f32, 9>(
        -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0,
    );

    let step = select(
        vec2<f32>(0.0, 1.0 / max(f32(u.fb_height), 1.0)),
        vec2<f32>(1.0 / max(f32(u.fb_width), 1.0), 0.0),
        u.direction_is_x == 1u,
    );

    var acc = vec3<f32>(0.0);
    for (var i = 0u; i < 9u; i = i + 1u) {
        let uv = in.uv + step * offsets[i];
        let sample = textureSample(input_tex, input_sampler, uv).rgb;
        // Bright-pass on H pass only — the V pass reads the H pass's
        // output which already had the threshold applied, so re-
        // thresholding would over-cull.
        let contrib = select(
            sample,
            bright_pass(sample, u.glow_threshold),
            u.direction_is_x == 1u,
        );
        acc = acc + contrib * weights[i];
    }
    return vec4<f32>(acc, 1.0);
}
