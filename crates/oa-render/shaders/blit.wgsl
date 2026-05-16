// oa-render — blit.wgsl
//
// The minimal output shader: stretch a 2D RGBA texture across the swapchain
// using the standard fullscreen-triangle vertex trick (no vertex buffer).
//
// Later phases layer scanline / CRT / phosphor / bezel passes on top; this stage
// is always the final write into the surface.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)        uv:       vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // One oversized triangle covering the screen with UV in [0..1] over the
    // visible NDC region.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        // Texture origin is top-left; NDC y goes up. Flip y so the framebuffer
        // displays right-side-up.
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(positions[idx], 0.0, 1.0);
    out.uv = uvs[idx];
    return out;
}

@group(0) @binding(0) var framebuffer: texture_2d<f32>;
@group(0) @binding(1) var framebuffer_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(framebuffer, framebuffer_sampler, in.uv);
}
