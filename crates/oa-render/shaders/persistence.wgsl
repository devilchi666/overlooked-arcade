// oa-render — persistence.wgsl
//
// Phosphor-decay accumulator for the VectorPhosphor preset chain.
// Reads two textures — the current frame's bright-passed Gaussian
// bloom output and the prior frame's accumulated history — and
// writes the next history slot:
//
//   history_curr = current_glow + history_prev * decay
//
// At 60fps + ~80ms half-life, the decay constant is 0.5^(16.67/80) ≈
// 0.866. Each frame the prior history is multiplied by 0.866, then
// the current glow is added. After ~5 frames the contribution from
// a one-shot vector stroke has decayed to half intensity; after ~10
// it's effectively gone. That matches the visible "vector ghosting"
// real Vectrex players remember when a bright stroke moves quickly.
//
// The pipeline reuses the 5-entry final_blit_bgl layout (tex0/
// sampler0/uniform/tex1/sampler1) so we don't add a new BGL to the
// renderer. `current_glow` lives at binding 0 + 1 (the layout's
// primary slot); `history_prev` lives at binding 3 + 4 (the layout's
// secondary slot). The uniform at binding 2 carries the decay
// constant.

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
    /// Per-frame decay factor — multiplied into the prior history
    /// before adding the current glow contribution. 0.866 ≈ 80ms
    /// half-life at 60fps. Variable so future "persistence
    /// half-life" overrides can write a different value at runtime.
    decay: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var current_glow: texture_2d<f32>;
@group(0) @binding(1) var current_sampler: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;
@group(0) @binding(3) var history_prev: texture_2d<f32>;
@group(0) @binding(4) var history_prev_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let curr = textureSample(current_glow, current_sampler, in.uv).rgb;
    let prev = textureSample(history_prev, history_prev_sampler, in.uv).rgb;
    // Cap the accumulated value so a string of bright strokes can't
    // overflow into HDR-level brightness over many frames. The cap is
    // generous (2.0 = double the source's max) so legitimate "bright
    // stroke holding still" still saturates the chip cleanly.
    let acc = curr + prev * u.decay;
    return vec4<f32>(min(acc, vec3<f32>(2.0)), 1.0);
}
