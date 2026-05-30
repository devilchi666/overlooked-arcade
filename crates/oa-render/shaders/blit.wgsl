// oa-render — blit.wgsl
//
// Phase 1: minimal output shader — stretch a 2D RGBA texture across the
// swapchain using the standard fullscreen-triangle vertex trick (no vertex
// buffer).
//
// Phase 3 slice A: the fragment shader branches on a `preset_id` from a
// small uniform buffer. Slice B added a multi-pass chain (intermediate
// render targets) for effects that need a separable / multi-tap kernel.
// Slice B-2 added a second texture binding so the final blit can sample
// BOTH the source framebuffer AND a chain output — that's what the
// Phosphor composite needs (`mix(source, blur, bloom_amount)`).
//
// Presets:
//   0 = Plain        — pass-through (Phase 1 baseline). Samples slot 0 only.
//   1 = Scanlines    — alternate-row darken at the source-pixel rate. The
//                      scanline period locks to fb_height so it stays crisp at
//                      any output resolution (the rasterizer's UV interp gives
//                      us a continuous coordinate we round to source rows).
//   2 = CrtLite      — Scanlines + radial vignette + a small saturation lift
//                      to recover the perceived dimming. Not physically
//                      accurate — just visually distinct from Plain.
//   3 = Phosphor     — Slice B-2 composite. Slot 0 is the source framebuffer,
//                      slot 3 is the blurred chain output (H-blur then V-blur
//                      from blur.wgsl). Returns `mix(src, blur, bloom_amount)`.
//                      For other presets slot 3 is bound to the same source
//                      framebuffer as slot 0 so the binding is always valid
//                      even when unused.
//   4 = LcdHandheld  — Slice E. Simulates an LCD's R/G/B subpixel triplet by
//                      tinting horizontal sub-stripes of each source pixel
//                      (3 stripes per source pixel: red column zeroes G/B
//                      partly, green keeps G, etc.). Adds a faint inter-pixel
//                      grid to hint at the matrix gap. NO scanlines — LCDs
//                      don't have them. Designed for gb (160x144), gba
//                      (240x160), gg (160x144), ngp (160x152), ws (224x144).
//   5 = VectorPhosphor — Vectrex vector-CRT. Slot 3 carries the wider-σ
//                      bright-pass blur output from vector_blur.wgsl (and
//                      will carry the persistence-accumulated history in
//                      P2). Final composite is `source + glow * bloom_amount`,
//                      additive so bright vector strokes punch over the
//                      black background; bloom_amount doubles as the glow
//                      strength knob (default 1.0 = full halo).
//   6 = VbMonochrome — Virtual Boy LED scanner. Vertical scanline darken
//                      (mimics the VB's spinning-mirror LED column) +
//                      soft circular vignette (mimics the headset eyepiece
//                      framing) + a red-saturation lift that crushes any
//                      residual green/blue out so the palette stays pure
//                      red-on-black. Single-pass; no chain or persistence.

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

struct Uniforms {
    /// 0 = Plain · 1 = Scanlines · 2 = CrtLite · 3 = Phosphor composite.
    /// Anything else falls through to Plain so an unrecognized preset id
    /// can't crash a draw.
    preset_id: u32,
    /// Source framebuffer height in pixels. Used to lock the scanline period
    /// to the source row rate regardless of output resolution.
    fb_height: u32,
    /// Slice B-2 — Phosphor composite weight in [0, 1]. `0` = pure source
    /// (no bloom), `1` = pure blur. Default 0.6 ships; slice C surfaces a
    /// slider via the TOML preset format. Ignored by every preset that
    /// isn't preset_id == 3.
    bloom_amount: f32,
    /// Display rotation in 90°-clockwise units (0..=3). Pushed by the
    /// shell from RETRO_ENVIRONMENT_SET_ROTATION; non-zero on vertical
    /// arcade boards (Pac-Man, Galaxian, DK). Applied to UV before
    /// texture sampling so the source image appears rotated on screen.
    rotation: u32,
    /// Overscan crop in source UV space — texture-sample UV gets
    /// remapped from `[0..1]` to `[uv_min..uv_max]`. Default `(0,0,1,1)`
    /// = no crop. The destination viewport is unchanged, so cropped
    /// pixels are stretched to fill the visible viewport (the typical
    /// "hide TV-overscan edges + zoom to fill" behaviour).
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
};

@group(0) @binding(0) var framebuffer: texture_2d<f32>;
@group(0) @binding(1) var framebuffer_sampler: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;
// Slice B-2 — secondary input for composite presets. For Phosphor this
// is the V-blur chain output; for everything else it's a duplicate of
// the framebuffer binding so the slot is valid but unused.
@group(0) @binding(3) var secondary: texture_2d<f32>;
@group(0) @binding(4) var secondary_sampler: sampler;

fn apply_scanlines(base: vec4<f32>, uv: vec2<f32>, fb_h: f32, intensity: f32) -> vec4<f32> {
    // Round to source row and toggle. `intensity` is the darken factor for
    // the "off" rows (e.g. 0.85 = 15% darker).
    let row = u32(uv.y * fb_h);
    let darken = select(intensity, 1.0, (row % 2u) == 0u);
    return vec4<f32>(base.rgb * darken, base.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Apply display rotation BEFORE the overscan-crop UV remap so the
    // crop bounds stay aligned with the source's natural orientation.
    // For destination pixel at screen UV (x, y) under rotation R
    // (90° CW units):
    //   0: src = (x, y)               [identity]
    //   1: src = (y, 1-x)             [90° CW — vertical arcade board on landscape]
    //   2: src = (1-x, 1-y)           [180°]
    //   3: src = (1-y, x)             [270° CW = 90° CCW]
    var rotated_uv: vec2<f32> = in.uv;
    if (u.rotation == 1u) {
        rotated_uv = vec2<f32>(in.uv.y, 1.0 - in.uv.x);
    } else if (u.rotation == 2u) {
        rotated_uv = vec2<f32>(1.0 - in.uv.x, 1.0 - in.uv.y);
    } else if (u.rotation == 3u) {
        rotated_uv = vec2<f32>(1.0 - in.uv.y, in.uv.x);
    }
    // Remap rotated UV [0..1] → [uv_min..uv_max] for texture sampling.
    // The default (0,0,1,1) leaves sampling unchanged. With an overscan
    // crop the sample_uv stays inside the crop bounds; the destination
    // viewport is unchanged so the cropped region stretches to fill.
    let sample_uv = mix(u.uv_min, u.uv_max, rotated_uv);
    let base = textureSample(framebuffer, framebuffer_sampler, sample_uv);
    let fb_h = max(f32(u.fb_height), 1.0);

    if (u.preset_id == 1u) {
        // Scanlines key off the SAMPLED source row so the period stays
        // aligned with the original framebuffer rows even when cropped.
        return apply_scanlines(base, sample_uv, fb_h, 0.85);
    }
    if (u.preset_id == 2u) {
        // CrtLite — heavier scanlines + radial vignette + saturation lift.
        // Vignette uses the screen-space [0..1] in.uv so it stays
        // centered on the visible viewport regardless of source crop.
        let scanned = apply_scanlines(base, sample_uv, fb_h, 0.75);
        // Radial vignette around the visible UV center (0.5, 0.5). Soft
        // falloff that doesn't crush the corners.
        let cx = in.uv.x - 0.5;
        let cy = in.uv.y - 0.5;
        let r = sqrt(cx * cx + cy * cy);
        let vignette = 1.0 - clamp(r * 0.55, 0.0, 0.35);
        // Light saturation lift to compensate for the dimming. Pull rgb
        // toward its luminance only if vignette pushed below 1.
        let lum = dot(scanned.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let saturated = mix(vec3<f32>(lum), scanned.rgb, 1.1);
        return vec4<f32>(saturated * vignette, scanned.a);
    }
    if (u.preset_id == 3u) {
        // Phosphor composite (slice B-2). `base` is the source framebuffer;
        // `bloom` is the blurred chain output. The composite preserves
        // high-frequency detail from the source and adds a soft halo from
        // the blur — closer to a real phosphor's behavior than a pure-blur
        // pass would be. Bloom samples the chain output at the cropped
        // UV so both inputs cover the same source region.
        let bloom = textureSample(secondary, secondary_sampler, sample_uv);
        let amt = clamp(u.bloom_amount, 0.0, 1.0);
        return vec4<f32>(mix(base.rgb, bloom.rgb, amt), base.a);
    }
    if (u.preset_id == 4u) {
        // LCD-handheld (slice E). Per-source-pixel RGB subpixel triplet
        // tint + faint inter-pixel grid. The subpixel layout is the
        // simplest stripe arrangement: each source pixel × 3 horizontal
        // sub-stripes (R / G / B). Each fragment falls in one sub-stripe
        // determined by fract(src_x) * 3. The mask multiplies the source
        // color so the R stripe shows the R channel strongly and dims
        // G + B partially — what an LCD's actual subpixel does.
        //
        // A small brightness lift compensates for the dimming the
        // subpixel mask introduces; clamping at 1 keeps highlights from
        // blooming past white.
        let src_w = f32(textureDimensions(framebuffer).x);
        let src_x = sample_uv.x * src_w;
        let src_y = sample_uv.y * fb_h;
        let sub_idx = u32(fract(src_x) * 3.0);
        let masks: array<vec3<f32>, 3> = array<vec3<f32>, 3>(
            vec3<f32>(1.0, 0.5, 0.5),
            vec3<f32>(0.5, 1.0, 0.5),
            vec3<f32>(0.5, 0.5, 1.0),
        );
        let tinted = base.rgb * masks[sub_idx];
        // Inter-pixel grid: thin darker lines at every source-pixel
        // boundary. step(0.92, fract(x)) returns 1 in the rightmost ~8%
        // of each source pixel, 0 elsewhere.
        let grid_x = step(0.92, fract(src_x));
        let grid_y = step(0.92, fract(src_y));
        let grid = 1.0 - 0.25 * max(grid_x, grid_y);
        let lifted = min(tinted * grid * 1.3, vec3<f32>(1.0));
        return vec4<f32>(lifted, base.a);
    }
    if (u.preset_id == 5u) {
        // VectorPhosphor — additive glow composite. `base` is the
        // source (Vectrex's pure-black background + bright vector
        // strokes); `glow` is the wider-σ bright-passed blur from
        // vector_blur.wgsl. Additive over the source keeps strokes
        // crisp while painting a halo around them. `bloom_amount`
        // doubles as the glow strength knob — 0 = no halo (effectively
        // Plain), 1 = full Vectrex halo, >1 = extra punch.
        let glow = textureSample(secondary, secondary_sampler, sample_uv).rgb;
        let amt = clamp(u.bloom_amount, 0.0, 2.0);
        let composited = base.rgb + glow * amt;
        return vec4<f32>(min(composited, vec3<f32>(1.5)), base.a);
    }
    if (u.preset_id == 6u) {
        // VbMonochrome — Virtual Boy LED scanner aesthetic.
        //
        // Step 1: Pure-red palette enforcement. Mednafen VB renders in
        // red but the framebuffer is still RGB. Crush green/blue so a
        // future palette tweak or core option that introduces any
        // residual color doesn't violate the era-correct monochrome
        // look. The max() against the existing R channel keeps bright
        // pixels bright while ensuring G/B drop to zero.
        let red_only = vec3<f32>(base.r, 0.0, 0.0);
        // Step 2: Vertical scanline darkening — every other source
        // COLUMN gets dimmed to ~0.82× to mimic the spinning-mirror
        // LED column scanner. Locked to the source pixel rate via
        // textureDimensions so the artifact stays crisp at any output
        // resolution. VB native is 384×224; even columns are bright,
        // odd columns are the scanned-past dim state.
        let src_w = f32(textureDimensions(framebuffer).x);
        let src_col = u32(sample_uv.x * src_w);
        let col_dim = select(0.82, 1.0, (src_col % 2u) == 0u);
        let scanned = red_only * col_dim;
        // Step 3: Soft circular vignette — eyepiece framing. Smooth
        // radial falloff around the visible viewport center; ~0.7 at
        // the corners. Soft enough that gameplay reads clearly; just
        // enough to sell the "wearing the headset" framing.
        //
        // We use the screen-space in.uv (not sample_uv) so the
        // vignette stays centered on the visible viewport regardless
        // of overscan crop.
        let cx = in.uv.x - 0.5;
        let cy = in.uv.y - 0.5;
        let r = sqrt(cx * cx + cy * cy);
        // smoothstep falls from 1.0 inside r=0.35 to ~0.7 at r=0.7
        // (the corner of a unit square is r=sqrt(2)/2 ≈ 0.707).
        let vignette = 1.0 - smoothstep(0.35, 0.7, r) * 0.3;
        return vec4<f32>(scanned * vignette, base.a);
    }
    return base;
}
