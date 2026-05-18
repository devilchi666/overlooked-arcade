//! oa-render — wgpu-backed renderer.
//!
//! The [`Renderer`] owns a wgpu surface attached to a host-provided native window
//! and presents a core's [`oa_core::Framebuffer`] each frame. Phase 1: nearest-
//! neighbour blit through a fullscreen-triangle pass with no post-process.
//! Phase 3 will layer scanline / CRT-curve / phosphor / bezel passes between the
//! framebuffer texture and this final blit.
//!
//! Surface ownership follows the Spike 1 pattern: caller passes raw window +
//! display handles and guarantees (via an `Arc<Window>` held for the renderer's
//! lifetime) that the window outlives the renderer.

#![deny(rust_2018_idioms)]

use std::borrow::Cow;
use std::num::NonZeroU32;

use oa_core::Framebuffer;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Errors the renderer can surface during construction or present.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// No suitable wgpu adapter on this machine.
    #[error("no compatible wgpu adapter found")]
    NoAdapter,
    /// Failed to create the wgpu surface from the supplied handles.
    #[error("create_surface failed: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    /// Failed to request a wgpu device from the chosen adapter.
    #[error("request_device failed: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

/// How the core's framebuffer is mapped to the surface.
///
/// `AspectCorrectFit` is the default — it matches the Phase 1.5 behavior and is
/// the most forgiving for arbitrary window sizes. The Phase 2 settings UI lets
/// the user pick any of the others; viewport math is the only thing that
/// changes (no shader / pipeline / texture state churn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingMode {
    /// Largest integer N (≥1) such that N rows of source fit vertically AND
    /// `N * fb_h * display_aspect` columns fit horizontally. Crisp scanline
    /// alignment + correct display aspect. Letterbox / pillarbox the rest.
    PixelPerfect,
    /// Largest rectangle inside the surface that preserves the core's reported
    /// display aspect. Phase 1.5 default.
    AspectCorrectFit,
    /// Fill the entire surface; ignore aspect (may distort).
    Stretched,
    /// Native `fb_w × fb_h` centered in the surface. No scaling at all.
    Original,
    /// Fixed integer scale: `fb_w * N × fb_h * N` centered, regardless of fit.
    /// Letterbox / pillarbox if it doesn't fit; the viewport is clamped to the
    /// surface bounds (so the visible image is cropped, not just letterboxed).
    IntegerMultiple(u32),
}

impl Default for ScalingMode {
    fn default() -> Self {
        Self::AspectCorrectFit
    }
}

/// Selectable shader preset applied during the final blit.
///
/// Phase 3 slice A ships three presets in a single fragment shader that
/// branches on the `preset_id` uniform. Slice B onward adds multi-pass
/// effects (separable Gaussian for the phosphor bloom, the bezel overlay
/// composite, etc.) that don't fit a one-shot branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderPreset {
    /// Pass-through. Phase 1 baseline.
    Plain,
    /// Alternate-row darken at the source-pixel rate. Single-pass.
    Scanlines,
    /// Scanlines + radial vignette + saturation lift. Stylized, not accurate.
    /// Single-pass.
    CrtLite,
    /// Phase 3 slice B — phosphor decay simulation via 2-pass separable
    /// Gaussian blur. Crude approximation: phosphors leak light to
    /// neighbouring pixels; a small blur of the framebuffer produces a
    /// recognizable "soft CRT" look without paying for a full 2D kernel
    /// per fragment. Multi-pass — exercises the EffectPass chain.
    Phosphor,
}

impl Default for ShaderPreset {
    fn default() -> Self {
        Self::Plain
    }
}

impl ShaderPreset {
    /// Stable id used in the WGSL uniform branch + the per-game / per-system
    /// settings persistence layers. Phosphor is `3` — slice B-2 added a
    /// composite branch that samples BOTH the source (slot 0) and the
    /// chain's blur output (slot 3) inside the final blit, so the final
    /// blit DOES work for Phosphor (it didn't before slice B-2).
    pub fn id(self) -> u32 {
        match self {
            Self::Plain => 0,
            Self::Scanlines => 1,
            Self::CrtLite => 2,
            Self::Phosphor => 3,
        }
    }

    /// Parse the frontend's canonical preset string. Unknown values fall back
    /// to `Plain` so a stale persisted preset can't crash the renderer.
    pub fn parse(s: &str) -> Self {
        match s {
            "scanlines" => Self::Scanlines,
            "crt-lite" => Self::CrtLite,
            "phosphor" => Self::Phosphor,
            _ => Self::Plain,
        }
    }

    /// String the frontend persists for this preset.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Scanlines => "scanlines",
            Self::CrtLite => "crt-lite",
            Self::Phosphor => "phosphor",
        }
    }

    /// True for presets that require intermediate render targets (the
    /// `effect_chain`) to be populated before the final blit reads them.
    /// Used by `present()` to decide whether to allocate + run the chain.
    pub fn is_multipass(self) -> bool {
        matches!(self, Self::Phosphor)
    }
}

/// The wgpu blit renderer. One per game window.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Final blit to the swapchain. Reads from either `fb_texture` (no
    /// multi-pass chain) or the last intermediate (multi-pass chain ran)
    /// and applies the preset-id branch + viewport math.
    pipeline: wgpu::RenderPipeline,
    /// 3-entry layout (tex / sampler / uniform). Chain passes (e.g. blur)
    /// use this — they only need to sample one input. Stays at 3 entries
    /// so adding bindings to the final blit (slice B-2) doesn't pollute
    /// the chain-pass bind group construction.
    bind_group_layout: wgpu::BindGroupLayout,
    /// 5-entry layout (tex0 / sampler0 / uniform / tex1 / sampler1). Used
    /// by the final blit pipeline and `fb_texture.bind_group`. The
    /// secondary slots carry the chain output for composite presets
    /// (Phosphor) and a duplicate of slot 0 for single-pass presets.
    final_blit_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Tiny uniform buffer carrying { preset_id, fb_height, _pad, _pad }.
    /// Updated each `present` before the draw — the cost is negligible
    /// against per-frame texture upload of the framebuffer itself.
    uniform_buffer: wgpu::Buffer,
    // Lazily allocated when the first framebuffer with a known size arrives.
    fb_texture: Option<FbTexture>,
    /// Phase 3 slice B — multi-pass effect chain infrastructure. Each entry
    /// is one pass that reads from the previous output (fb_texture for the
    /// first pass, intermediate_a/b ping-pong for later passes) and writes
    /// to the next intermediate. The final blit then samples the last
    /// intermediate. Empty Vec = no chain (Plain / Scanlines / CrtLite —
    /// they apply effects in the final-blit fragment shader directly).
    effect_chain: Vec<EffectPass>,
    /// Ping-pong intermediates sized to the framebuffer. Lazily allocated
    /// when the first multi-pass preset runs; reallocated if the
    /// framebuffer mode changes dimensions.
    intermediates: Option<(IntermediateTexture, IntermediateTexture)>,
    /// Phase 3 slice B-2 — bezel overlay. When set, an alpha-blend pass
    /// runs after the final blit drawing this RGBA texture stretched over
    /// the full surface. Renderer-side infrastructure ships in slice B-2;
    /// the shell-side path that loads bezel PNGs from per-system or
    /// per-game settings rides with slice C's TOML preset format.
    bezel: Option<BezelTexture>,
    bezel_pipeline: wgpu::RenderPipeline,
    bezel_bgl: wgpu::BindGroupLayout,
    /// Phase 3 slice B-2 — Phosphor composite weight in [0, 1]. Written to
    /// the final-blit uniform every present; the shader ignores it for
    /// non-composite presets. Default 0.6; slice C surfaces a slider via
    /// the TOML preset format.
    bloom_amount: f32,
    frames_presented: u64,
    scaling_mode: ScalingMode,
    shader_preset: ShaderPreset,
}

/// A single shader pass in the multi-pass effect chain. Owns its pipeline +
/// a tiny uniform buffer; the bind group is built per-frame against the
/// current input texture (since the input rotates between fb_texture and
/// the ping-pong intermediates).
struct EffectPass {
    #[allow(dead_code)] // used for wgpu labels in debug builds + RenderDoc
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    /// 16-byte uniform payload written into `uniform_buffer` once on chain
    /// build. Persisted here so a per-frame fb-dimension write can re-pack
    /// the same layout without holding the original parameters as fields.
    /// Layout depends on the shader — for the blur pass it's
    /// `[direction_is_x, fb_width, fb_height, _pad]` as u32.
    uniform_bytes: [u32; 4],
}

struct IntermediateTexture {
    width: u32,
    height: u32,
    /// Held alongside `view` to keep the underlying GPU resource alive +
    /// available for future RenderDoc labels / readback paths. Currently
    /// only `view` is bound into pipelines; the texture handle is just a
    /// retainer. Same shape as `BezelTexture`.
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Loaded bezel artwork. Texture is RGBA8, sRGB color space (matches the
/// swapchain); alpha drives the blend over the underlying game pixels.
struct BezelTexture {
    width: u32,
    height: u32,
    #[allow(dead_code)] // diagnostics + future "show bezel info" UI
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

struct FbTexture {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    /// View kept alongside the texture so chain passes can create their own
    /// bind groups against it without re-creating the view per frame.
    view: wgpu::TextureView,
    /// Bind group bound for the single-pass final blit path (final blit
    /// reads fb_texture directly when the effect chain is empty).
    bind_group: wgpu::BindGroup,
}

impl Renderer {
    /// Construct a renderer against the supplied native window.
    ///
    /// # Safety
    /// `window_handle` and `display_handle` must refer to a live window that
    /// outlives this `Renderer`. The shell is expected to keep an `Arc<Window>`
    /// alive for the renderer's lifetime.
    pub unsafe fn new(
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
        size: (u32, u32),
    ) -> Result<Self, RenderError> {
        pollster::block_on(Self::new_async(window_handle, display_handle, size))
    }

    async fn new_async(
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
        (width, height): (u32, u32),
    ) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // SAFETY: caller's contract — see Renderer::new docs.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_window_handle: window_handle,
                raw_display_handle: display_handle,
            })?
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::NoAdapter)?;
        log::info!("oa-render: adapter = {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("oa-render device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let alpha_mode = caps.alpha_modes[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        log::info!("oa-render: surface configured ({}x{}, {:?})", config.width, config.height, format);

        // Chain-pass bind group layout: 0 = framebuffer texture, 1 = sampler,
        // 2 = preset uniform. Used by every effect-chain pass (currently
        // just the H/V blur in blur.wgsl).
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oa-render bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Final-blit bind group layout: chain layout + bindings 3, 4 for a
        // secondary texture / sampler. Slice B-2's Phosphor composite needs
        // to sample BOTH the source framebuffer (slot 0) AND the chain's
        // blur output (slot 3) inside one fragment shader; this layout
        // declares the slots that branch uses. Non-composite presets bind
        // slot 3 + 4 to the same fb_view + sampler — the slots are valid
        // but the shader ignores them.
        let final_blit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oa-render final-blit bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // 16-byte uniform: { preset_id: u32, fb_height: u32, _pad0: u32, _pad1: u32 }.
        // wgpu requires uniform buffers be a multiple of 16 bytes; we round
        // up by carrying two pad slots.
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oa-render preset uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("oa-render fb sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // Nearest = crisp pixel art. Phase 3 makes this configurable.
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oa-render blit shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/blit.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oa-render pipeline layout"),
            bind_group_layouts: &[&final_blit_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oa-render blit pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Bezel pipeline (slice B-2). 2-entry layout (tex + sampler); no
        // uniform. Alpha-blends the bezel texture over the swapchain in a
        // load-existing-content render pass that runs after the main blit.
        let bezel_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oa-render bezel bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bezel_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oa-render bezel shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/bezel.wgsl"))),
        });
        let bezel_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oa-render bezel pipeline layout"),
            bind_group_layouts: &[&bezel_bgl],
            push_constant_ranges: &[],
        });
        let bezel_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oa-render bezel pipeline"),
            layout: Some(&bezel_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bezel_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bezel_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    // Standard alpha composite: src.rgb * src.a + dst.rgb * (1 - src.a).
                    // Keeps the underlying game pixels visible where the
                    // bezel's alpha is zero.
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group_layout,
            final_blit_bgl,
            sampler,
            uniform_buffer,
            fb_texture: None,
            effect_chain: Vec::new(),
            intermediates: None,
            bezel: None,
            bezel_pipeline,
            bezel_bgl,
            bloom_amount: 0.6,
            frames_presented: 0,
            scaling_mode: ScalingMode::default(),
            shader_preset: ShaderPreset::default(),
        })
    }

    /// Load an RGBA8 bezel image (sRGB color space). Width × height pixels;
    /// `rgba.len()` must equal `width * height * 4`. Returns an error if the
    /// dimensions are zero or the byte length mismatches.
    ///
    /// The bezel is rendered alpha-blended over the surface AFTER the
    /// final blit on every subsequent `present()` until `clear_bezel_image`
    /// is called. The image is sized to the FULL surface (not the game
    /// viewport) — users design bezels to match their window dimensions.
    pub fn set_bezel_image(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err(format!("bezel dimensions must be non-zero (got {width}x{height})"));
        }
        let expected = (width as usize) * (height as usize) * 4;
        if rgba.len() != expected {
            return Err(format!(
                "bezel byte length {} doesn't match {}x{}x4 = {}",
                rgba.len(), width, height, expected,
            ));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oa-render bezel texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // sRGB so the swapchain's gamma-correct blend produces the
            // colors the bezel artist saw in their image editor.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes_per_row = NonZeroU32::new(width * 4).expect("width > 0 checked above");
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row.get()),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oa-render bezel bind group"),
            layout: &self.bezel_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        log::info!("oa-render: bezel image set {}x{} (sRGB RGBA8)", width, height);
        self.bezel = Some(BezelTexture { width, height, texture, bind_group });
        Ok(())
    }

    /// Remove the active bezel. Subsequent `present()` calls skip the
    /// overlay pass and the surface shows the bare game blit again.
    pub fn clear_bezel_image(&mut self) {
        if self.bezel.is_some() {
            log::info!("oa-render: bezel image cleared");
        }
        self.bezel = None;
    }

    /// Whether a bezel is currently loaded. For diagnostics + tests.
    pub fn has_bezel(&self) -> bool {
        self.bezel.is_some()
    }

    /// Dimensions of the active bezel, or `None` if no bezel is set.
    pub fn bezel_dimensions(&self) -> Option<(u32, u32)> {
        self.bezel.as_ref().map(|b| (b.width, b.height))
    }

    /// Pick a different shader preset. Takes effect on the next `present()`.
    /// Rebuilds the effect chain — multi-pass presets get one or more
    /// `EffectPass` entries, single-pass presets clear the chain back to
    /// empty (effects apply in the final-blit fragment shader's preset
    /// branch instead).
    pub fn set_shader_preset(&mut self, preset: ShaderPreset) {
        if self.shader_preset != preset {
            log::info!("oa-render: shader preset {:?} -> {:?}", self.shader_preset, preset);
            self.shader_preset = preset;
            self.effect_chain = self.build_effect_chain(preset);
        }
    }

    /// Construct the EffectPass sequence for a given preset. Called from
    /// `set_shader_preset`; single-pass presets return an empty Vec.
    fn build_effect_chain(&self, preset: ShaderPreset) -> Vec<EffectPass> {
        match preset {
            ShaderPreset::Phosphor => {
                let mut chain = Vec::with_capacity(2);
                chain.push(self.create_blur_pass("phosphor h-blur", true));
                chain.push(self.create_blur_pass("phosphor v-blur", false));
                chain
            }
            // Single-pass presets — effects apply in the final-blit shader's
            // preset branch via the uniform_buffer's preset_id field.
            ShaderPreset::Plain | ShaderPreset::Scanlines | ShaderPreset::CrtLite => Vec::new(),
        }
    }

    /// Build a separable-Gaussian blur pass. `direction_is_x = true` runs
    /// the horizontal axis; false runs vertical. Both passes share the same
    /// WGSL shader, differing only in the uniform's direction flag.
    fn create_blur_pass(&self, label: &'static str, direction_is_x: bool) -> EffectPass {
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oa-render blur shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/blur.wgsl"))),
        });
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("oa-render blur pipeline layout"),
            bind_group_layouts: &[&self.bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                // Intermediate textures are Rgba8Unorm (linear), not the
                // sRGB swapchain format. Pre-blit effects happen in a
                // linear space so the final blit's sRGB encode is the only
                // gamma transition.
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oa-render blur uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        EffectPass {
            label,
            pipeline,
            uniform_buffer,
            // fb_width / fb_height are written every present(); the direction
            // flag is the only constant-per-pass field.
            uniform_bytes: [if direction_is_x { 1 } else { 0 }, 0, 0, 0],
        }
    }

    /// Allocate (or reallocate) the ping-pong intermediate pair at the
    /// supplied framebuffer dimensions. Idempotent on dimension match.
    fn ensure_intermediates(&mut self, width: u32, height: u32) {
        let needs_alloc = match &self.intermediates {
            Some((a, _)) => a.width != width || a.height != height,
            None => true,
        };
        if !needs_alloc {
            return;
        }
        let a = self.create_intermediate("oa-render intermediate A", width, height);
        let b = self.create_intermediate("oa-render intermediate B", width, height);
        log::info!("oa-render: intermediates allocated {}x{} (RGBA8 linear)", width, height);
        self.intermediates = Some((a, b));
    }

    fn create_intermediate(&self, label: &'static str, width: u32, height: u32) -> IntermediateTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Linear RGBA8 — the chain works in linear space, the final
            // blit handles the sRGB encode into the swapchain.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        IntermediateTexture { width, height, texture, view }
    }

    /// The currently active shader preset.
    pub fn shader_preset(&self) -> ShaderPreset {
        self.shader_preset
    }

    /// Set the Phosphor composite weight (clamped to [0, 1]). `0.0` = pure
    /// source (the chain still runs but its output is invisible), `1.0` =
    /// pure blur (no source contribution — visually equivalent to the
    /// pre-slice-B-2 Phosphor preset). Takes effect on the next present().
    pub fn set_bloom_amount(&mut self, amount: f32) {
        let clamped = amount.clamp(0.0, 1.0);
        if (clamped - self.bloom_amount).abs() > f32::EPSILON {
            log::info!("oa-render: bloom_amount {:.3} -> {:.3}", self.bloom_amount, clamped);
            self.bloom_amount = clamped;
        }
    }

    /// The currently active Phosphor bloom amount.
    pub fn bloom_amount(&self) -> f32 {
        self.bloom_amount
    }

    /// Pick a different scaling mode. Takes effect on the next `present()`.
    pub fn set_scaling_mode(&mut self, mode: ScalingMode) {
        if self.scaling_mode != mode {
            log::info!("oa-render: scaling mode {:?} -> {:?}", self.scaling_mode, mode);
            self.scaling_mode = mode;
        }
    }

    /// The currently active scaling mode.
    pub fn scaling_mode(&self) -> ScalingMode {
        self.scaling_mode
    }

    /// Update the surface dimensions when the window resizes.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if width == self.config.width && height == self.config.height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        log::debug!("oa-render: resized to {}x{}", width, height);
    }

    /// Upload `fb` and present one frame. If the active preset is multi-
    /// pass, the effect chain runs first into the ping-pong intermediates;
    /// the final blit then reads from whichever intermediate held the last
    /// chain output. Single-pass presets skip the chain entirely (effects
    /// apply in the final-blit shader's `preset_id` branch).
    pub fn present(&mut self, fb: Framebuffer<'_>) {
        // (Re)allocate the framebuffer texture if dimensions changed.
        let need_new_tex = match &self.fb_texture {
            Some(t) => t.width != fb.width || t.height != fb.height,
            None => true,
        };
        if need_new_tex {
            self.fb_texture = Some(self.create_fb_texture(fb.width, fb.height));
        }

        // Update the final-blit uniform. Layout matches the WGSL struct:
        // [preset_id u32, fb_height u32, bloom_amount f32, _pad u32]. The
        // bloom_amount slot is reinterpreted as f32 on the GPU side via
        // the WGSL declaration; we write the raw u32 bit-pattern here.
        let final_uniform: [u32; 4] = [
            self.shader_preset.id(),
            fb.height,
            self.bloom_amount.to_bits(),
            0,
        ];
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&final_uniform),
        );

        // Upload pixel bytes into the framebuffer texture.
        {
            let fb_tex = self.fb_texture.as_ref().expect("just initialised");
            let bytes_per_row = NonZeroU32::new(fb.width * 4).expect("framebuffer width must be > 0");
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &fb_tex.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                fb.pixels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row.get()),
                    rows_per_image: Some(fb.height),
                },
                wgpu::Extent3d {
                    width: fb.width,
                    height: fb.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        // === Multi-pass effect chain ====================================
        //
        // For Phosphor (2 passes) the dataflow is:
        //   pass 0 (H-blur): fb_texture → intermediate_a
        //   pass 1 (V-blur): intermediate_a → intermediate_b
        // Final blit then reads intermediate_b. For an N-pass chain in
        // general, pass i writes to intermediate_a if i is even else
        // intermediate_b; the last write is what the final blit samples.
        let last_intermediate_was_a = if !self.effect_chain.is_empty() {
            self.ensure_intermediates(fb.width, fb.height);

            // Re-pack each pass's uniform with the current fb dims while
            // preserving the constant-per-pass fields (e.g. direction flag).
            for pass in &self.effect_chain {
                let mut bytes = pass.uniform_bytes;
                bytes[1] = fb.width;
                bytes[2] = fb.height;
                self.queue.write_buffer(&pass.uniform_buffer, 0, bytemuck::cast_slice(&bytes));
            }
            self.run_effect_chain()
        } else {
            // Sentinel — final blit will read fb_texture directly.
            false
        };

        // === Final blit to swapchain ====================================
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                log::warn!("oa-render: get_current_texture failed: {e:?}");
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let final_bind_group = if self.effect_chain.is_empty() {
            // Single-pass path: the fb_texture's pre-built bind group is
            // fine — slot 0 = source, slots 3 + 4 = source again (the
            // shader ignores them for non-composite presets).
            None
        } else {
            // Multi-pass path: build a fresh bind group. Slot 0 = source
            // framebuffer, slot 3 = chain output. The Phosphor composite
            // branch in blit.wgsl samples both. For any future multi-pass
            // preset that DOESN'T composite (the chain output IS the final
            // pixels), the chain's last pass should write its result to
            // the fb-texture-equivalent path — currently no such preset
            // exists, but if one is added the shader branch decides what
            // to do with the two textures, not this binding.
            let fb_tex = self.fb_texture.as_ref().expect("fb_texture ready");
            let (a, b) = self.intermediates.as_ref().expect("ensure_intermediates ran");
            let chain_output_view = if last_intermediate_was_a { &a.view } else { &b.view };
            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("oa-render final-blit bind group (source + chain output)"),
                layout: &self.final_blit_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fb_tex.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.uniform_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(chain_output_view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            }))
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("oa-render encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oa-render blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            let fb_tex = self.fb_texture.as_ref().expect("fb_texture ready");
            let bind_group = final_bind_group.as_ref().unwrap_or(&fb_tex.bind_group);
            pass.set_bind_group(0, bind_group, &[]);
            let (vp_x, vp_y, vp_w, vp_h) = viewport_for(
                self.scaling_mode,
                self.config.width,
                self.config.height,
                fb.width,
                fb.height,
                fb.display_aspect,
            );
            pass.set_viewport(vp_x, vp_y, vp_w, vp_h, 0.0, 1.0);
            pass.draw(0..3, 0..1);
        }

        // === Bezel overlay pass (slice B-2) =============================
        //
        // Runs only when a bezel image is loaded. `LoadOp::Load` preserves
        // the game pixels we just blitted; the pipeline's alpha-blend
        // state composites the bezel over them. No viewport set — the
        // bezel covers the full surface.
        if let Some(bezel) = &self.bezel {
            let mut bezel_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oa-render bezel pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            bezel_pass.set_pipeline(&self.bezel_pipeline);
            bezel_pass.set_bind_group(0, &bezel.bind_group, &[]);
            bezel_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        self.frames_presented = self.frames_presented.wrapping_add(1);
    }

    /// Run every entry in the effect chain into the ping-pong intermediates.
    /// Returns `true` if the LAST written texture is intermediate_a, `false`
    /// if it's intermediate_b — the caller uses this flag to pick the right
    /// input view for the final blit.
    fn run_effect_chain(&self) -> bool {
        let fb_tex = self.fb_texture.as_ref().expect("fb_texture exists");
        let (a, b) = self.intermediates.as_ref().expect("intermediates allocated");

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("oa-render chain encoder"),
        });

        // Pre-compute each pass's bind group + output view.
        // Pass i: input = fb_view (if i==0) else (a or b alternating),
        //         output = intermediate_a (if i even) else intermediate_b.
        let mut last_was_a = false;
        for (i, pass) in self.effect_chain.iter().enumerate() {
            let input_view: &wgpu::TextureView = if i == 0 {
                &fb_tex.view
            } else if i % 2 == 1 {
                // Previous pass wrote to A (i-1 was even).
                &a.view
            } else {
                &b.view
            };
            let output_view: &wgpu::TextureView = if i % 2 == 0 { &a.view } else { &b.view };
            last_was_a = i % 2 == 0;

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("oa-render chain pass bind group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: pass.uniform_buffer.as_entire_binding() },
                ],
            });

            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(pass.label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&pass.pipeline);
            rp.set_bind_group(0, &bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        last_was_a
    }

    /// Frames presented since construction. For logging / diagnostics.
    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }

    /// Present a solid black frame. Used between ROM unload and the next ROM
    /// load — without it the wgpu swap chain keeps displaying the last
    /// framebuffer of the previous game, which is visually misleading once
    /// the user has explicitly unloaded.
    pub fn present_blank(&mut self) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                log::warn!("oa-render: get_current_texture failed (blank): {e:?}");
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("oa-render blank encoder") });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oa-render blank pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        self.frames_presented = self.frames_presented.wrapping_add(1);
    }

    fn create_fb_texture(&self, width: u32, height: u32) -> FbTexture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oa-render fb texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // fb_texture.bind_group is the single-pass shortcut — when the chain
        // is empty the final blit reads fb_texture directly. Slots 3 + 4
        // duplicate slot 0 + 1 so the 5-slot layout is satisfied but the
        // shader's composite branch (only fires for Phosphor) sees no
        // difference between slot 0 and slot 3.
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oa-render fb bind group"),
            layout: &self.final_blit_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        log::info!("oa-render: allocated fb texture {}x{} (RGBA8)", width, height);
        FbTexture { width, height, texture, view, bind_group }
    }
}

/// Compute the viewport rect (x, y, w, h) inside a `surface_w × surface_h`
/// drawing area for a given [`ScalingMode`]. The renderer relies on the
/// surrounding render-pass clear (black) to fill whatever the viewport doesn't
/// cover.
///
/// `display_aspect <= 0.0` falls back to `fb_w / fb_h` (square-pixel cores).
pub fn viewport_for(
    mode: ScalingMode,
    surface_w: u32,
    surface_h: u32,
    fb_w: u32,
    fb_h: u32,
    display_aspect: f32,
) -> (f32, f32, f32, f32) {
    let sw = surface_w.max(1) as f32;
    let sh = surface_h.max(1) as f32;
    let fb_w_f = fb_w.max(1) as f32;
    let fb_h_f = fb_h.max(1) as f32;
    let aspect = if display_aspect > 0.0 { display_aspect } else { fb_w_f / fb_h_f };

    match mode {
        ScalingMode::Stretched => (0.0, 0.0, sw, sh),
        ScalingMode::Original => center_rect(sw, sh, fb_w_f, fb_h_f),
        ScalingMode::IntegerMultiple(n) => {
            let n = n.max(1) as f32;
            center_rect(sw, sh, fb_w_f * n, fb_h_f * n)
        }
        ScalingMode::AspectCorrectFit => fit_aspect(sw, sh, aspect),
        ScalingMode::PixelPerfect => {
            // Largest integer N where N*fb_h fits vertically AND
            // N*fb_h*aspect fits horizontally. Vertical is always integer
            // (crisp scanlines); horizontal derives from aspect to preserve
            // the system's display aspect (PCE non-square pixels handled).
            let mut n: u32 = 1;
            loop {
                let next = (n + 1) as f32;
                let h = next * fb_h_f;
                let w = h * aspect;
                if h > sh || w > sw {
                    break;
                }
                n = (n + 1).min(64);
                if n == 64 { break; }
            }
            let h = (n as f32) * fb_h_f;
            let w = h * aspect;
            center_rect(sw, sh, w, h)
        }
    }
}

fn center_rect(sw: f32, sh: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let clipped_w = w.min(sw);
    let clipped_h = h.min(sh);
    let x = ((sw - clipped_w) * 0.5).max(0.0);
    let y = ((sh - clipped_h) * 0.5).max(0.0);
    (x, y, clipped_w, clipped_h)
}

fn fit_aspect(sw: f32, sh: f32, target_aspect: f32) -> (f32, f32, f32, f32) {
    let surface_aspect = sw / sh;
    if surface_aspect > target_aspect {
        // Surface is wider than the core wants — pillarbox left/right.
        let vp_h = sh;
        let vp_w = (sh * target_aspect).min(sw);
        let vp_x = ((sw - vp_w) * 0.5).max(0.0);
        (vp_x, 0.0, vp_w, vp_h)
    } else {
        // Surface is taller than the core wants — letterbox top/bottom.
        let vp_w = sw;
        let vp_h = (sw / target_aspect).min(sh);
        let vp_y = ((sh - vp_h) * 0.5).max(0.0);
        (0.0, vp_y, vp_w, vp_h)
    }
}

#[cfg(test)]
mod tests {
    use super::{viewport_for, ScalingMode, ShaderPreset};

    #[test]
    fn shader_preset_round_trips_strings() {
        assert_eq!(ShaderPreset::parse("plain"), ShaderPreset::Plain);
        assert_eq!(ShaderPreset::parse("scanlines"), ShaderPreset::Scanlines);
        assert_eq!(ShaderPreset::parse("crt-lite"), ShaderPreset::CrtLite);
        // Unknown strings fall back to Plain so a stale persisted preset
        // can't crash the renderer.
        assert_eq!(ShaderPreset::parse("nope"), ShaderPreset::Plain);
        assert_eq!(ShaderPreset::parse(""), ShaderPreset::Plain);
        // Round-trip via as_str().
        for p in [ShaderPreset::Plain, ShaderPreset::Scanlines, ShaderPreset::CrtLite] {
            assert_eq!(ShaderPreset::parse(p.as_str()), p);
        }
    }

    #[test]
    fn shader_preset_ids_are_stable() {
        // The WGSL `preset_id` uniform branches on these — locking them down
        // catches accidental reordering of the enum.
        assert_eq!(ShaderPreset::Plain.id(), 0);
        assert_eq!(ShaderPreset::Scanlines.id(), 1);
        assert_eq!(ShaderPreset::CrtLite.id(), 2);
        // Phosphor's final blit does real work now (slice B-2 composite):
        // it samples the source (slot 0) AND the blur output (slot 3) and
        // returns `mix(src, blur, bloom_amount)`. Id 3 is the WGSL branch.
        assert_eq!(ShaderPreset::Phosphor.id(), 3);
    }

    #[test]
    fn is_multipass_separates_chain_from_branch() {
        // Single-pass: effect applies in final-blit shader's preset branch.
        assert!(!ShaderPreset::Plain.is_multipass());
        assert!(!ShaderPreset::Scanlines.is_multipass());
        assert!(!ShaderPreset::CrtLite.is_multipass());
        // Multi-pass: needs intermediate render targets.
        assert!(ShaderPreset::Phosphor.is_multipass());
    }

    #[test]
    fn phosphor_string_round_trips() {
        assert_eq!(ShaderPreset::parse("phosphor"), ShaderPreset::Phosphor);
        assert_eq!(ShaderPreset::Phosphor.as_str(), "phosphor");
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.001
    }

    #[test]
    fn aspect_fit_wider_surface_pillarboxes() {
        let (x, y, w, h) = viewport_for(ScalingMode::AspectCorrectFit, 1920, 1080, 320, 240, 4.0 / 3.0);
        assert!(approx(y, 0.0));
        assert!(approx(h, 1080.0));
        assert!(approx(w, 1440.0));
        assert!(approx(x, 240.0));
    }

    #[test]
    fn aspect_fit_taller_surface_letterboxes() {
        let (x, y, w, h) = viewport_for(ScalingMode::AspectCorrectFit, 1080, 1920, 320, 240, 4.0 / 3.0);
        assert!(approx(x, 0.0));
        assert!(approx(w, 1080.0));
        assert!(approx(h, 810.0));
        assert!(approx(y, 555.0));
    }

    #[test]
    fn aspect_fit_matching_aspect_fills() {
        let (x, y, w, h) = viewport_for(ScalingMode::AspectCorrectFit, 800, 600, 320, 240, 4.0 / 3.0);
        assert!(approx(x, 0.0));
        assert!(approx(y, 0.0));
        assert!(approx(w, 800.0));
        assert!(approx(h, 600.0));
    }

    #[test]
    fn aspect_fit_zero_aspect_falls_back_to_fb_dims() {
        let (_x, _y, w, h) = viewport_for(ScalingMode::AspectCorrectFit, 1920, 1080, 256, 239, 0.0);
        assert!(approx(h, 1080.0));
        assert!(approx(w, 1080.0 * 256.0 / 239.0));
    }

    #[test]
    fn stretched_fills_whole_surface() {
        let (x, y, w, h) = viewport_for(ScalingMode::Stretched, 1920, 1080, 320, 240, 4.0 / 3.0);
        assert_eq!((x, y), (0.0, 0.0));
        assert!(approx(w, 1920.0));
        assert!(approx(h, 1080.0));
    }

    #[test]
    fn original_centers_native_size() {
        let (x, y, w, h) = viewport_for(ScalingMode::Original, 1920, 1080, 320, 240, 4.0 / 3.0);
        assert!(approx(w, 320.0));
        assert!(approx(h, 240.0));
        assert!(approx(x, (1920.0 - 320.0) * 0.5));
        assert!(approx(y, (1080.0 - 240.0) * 0.5));
    }

    #[test]
    fn integer_multiple_centers_scaled_size() {
        let (x, y, w, h) = viewport_for(ScalingMode::IntegerMultiple(3), 1920, 1080, 320, 240, 4.0 / 3.0);
        assert!(approx(w, 960.0));
        assert!(approx(h, 720.0));
        assert!(approx(x, (1920.0 - 960.0) * 0.5));
        assert!(approx(y, (1080.0 - 720.0) * 0.5));
    }

    #[test]
    fn integer_multiple_clamps_to_surface_when_too_large() {
        // 8x on 320x240 = 2560x1920; surface only 1920x1080. Clamp + center.
        let (x, y, w, h) = viewport_for(ScalingMode::IntegerMultiple(8), 1920, 1080, 320, 240, 4.0 / 3.0);
        assert!(approx(w, 1920.0));
        assert!(approx(h, 1080.0));
        assert_eq!((x, y), (0.0, 0.0));
    }

    #[test]
    fn pixel_perfect_picks_largest_integer_height() {
        // 1080 / 240 = 4.5 → N=4. PCE-style aspect-preserving: width = 4*240*(4/3) = 1280.
        let (_x, _y, w, h) = viewport_for(ScalingMode::PixelPerfect, 1920, 1080, 320, 240, 4.0 / 3.0);
        assert!(approx(h, 960.0));
        assert!(approx(w, 1280.0));
    }

    #[test]
    fn pixel_perfect_pce_native() {
        // 256x239 PCE on a 720p surface, square pixels (no display aspect).
        // 720 / 239 = 3.01 → N=3 vertically. 3 * 239 = 717h, 3 * 239 * (256/239) = 768w.
        let (_x, _y, w, h) = viewport_for(ScalingMode::PixelPerfect, 1280, 720, 256, 239, 0.0);
        assert!(approx(h, 717.0));
        assert!(approx(w, 768.0));
    }

    #[test]
    fn pixel_perfect_falls_back_to_n1_in_tiny_window() {
        // Surface 200x150, fb 320x240 — even N=1 overflows. The function still
        // returns a viewport (center_rect clamps to surface bounds) rather
        // than div-by-zero or hang.
        let (x, y, w, h) = viewport_for(ScalingMode::PixelPerfect, 200, 150, 320, 240, 4.0 / 3.0);
        assert!(approx(w, 200.0));
        assert!(approx(h, 150.0));
        assert_eq!((x, y), (0.0, 0.0));
    }
}
