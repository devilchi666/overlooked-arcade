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

/// The wgpu blit renderer. One per game window.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    // Lazily allocated when the first framebuffer with a known size arrives.
    fb_texture: Option<FbTexture>,
    frames_presented: u64,
}

struct FbTexture {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
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

        // Bind group: 0 = framebuffer texture (RGBA8 unfiltered float), 1 = sampler.
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
            ],
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
            bind_group_layouts: &[&bind_group_layout],
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

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group_layout,
            sampler,
            fb_texture: None,
            frames_presented: 0,
        })
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

    /// Upload `fb` and present one frame.
    pub fn present(&mut self, fb: Framebuffer<'_>) {
        // (Re)allocate the framebuffer texture if dimensions changed.
        let need_new_tex = match &self.fb_texture {
            Some(t) => t.width != fb.width || t.height != fb.height,
            None => true,
        };
        if need_new_tex {
            self.fb_texture = Some(self.create_fb_texture(fb.width, fb.height));
        }
        let fb_tex = self.fb_texture.as_ref().expect("just initialised");

        // Upload pixel bytes into the texture.
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

        // Acquire swap chain frame, draw, present.
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
            pass.set_bind_group(0, &fb_tex.bind_group, &[]);
            // Aspect-correct fit: largest rectangle inside the surface that
            // preserves the core's reported display aspect. The clear color
            // (black) fills the remaining surface, giving us free letterboxing.
            let (vp_x, vp_y, vp_w, vp_h) = fit_viewport(
                self.config.width,
                self.config.height,
                fb.width,
                fb.height,
                fb.display_aspect,
            );
            pass.set_viewport(vp_x, vp_y, vp_w, vp_h, 0.0, 1.0);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        self.frames_presented = self.frames_presented.wrapping_add(1);
    }

    /// Frames presented since construction. For logging / diagnostics.
    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
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
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oa-render fb bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        log::info!("oa-render: allocated fb texture {}x{} (RGBA8)", width, height);
        FbTexture { width, height, texture, bind_group }
    }
}

/// Aspect-correct fit: returns the (x, y, w, h) rect inside a `surface_w × surface_h`
/// drawing area that preserves the requested display aspect. `display_aspect <= 0.0`
/// falls back to `fb_w as f32 / fb_h as f32` (square-pixel cores).
///
/// The renderer relies on the surrounding render-pass clear (black) to draw the
/// letterbox / pillarbox bars in whatever the viewport doesn't cover.
fn fit_viewport(surface_w: u32, surface_h: u32, fb_w: u32, fb_h: u32, display_aspect: f32) -> (f32, f32, f32, f32) {
    let target_aspect = if display_aspect > 0.0 {
        display_aspect
    } else if fb_h > 0 {
        fb_w as f32 / fb_h as f32
    } else {
        1.0
    };
    let sw = surface_w.max(1) as f32;
    let sh = surface_h.max(1) as f32;
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
    use super::fit_viewport;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.001
    }

    #[test]
    fn wider_surface_pillarboxes() {
        // 16:9 surface (1920x1080), core wants 4:3.
        let (x, y, w, h) = fit_viewport(1920, 1080, 320, 240, 4.0 / 3.0);
        assert!(approx(y, 0.0));
        assert!(approx(h, 1080.0));
        assert!(approx(w, 1440.0));         // 1080 * 4/3
        assert!(approx(x, 240.0));          // (1920 - 1440) / 2
    }

    #[test]
    fn taller_surface_letterboxes() {
        // Portrait 9:16 surface, core wants 4:3.
        let (x, y, w, h) = fit_viewport(1080, 1920, 320, 240, 4.0 / 3.0);
        assert!(approx(x, 0.0));
        assert!(approx(w, 1080.0));
        assert!(approx(h, 810.0));          // 1080 / (4/3)
        assert!(approx(y, 555.0));          // (1920 - 810) / 2
    }

    #[test]
    fn matching_aspect_fills() {
        // Surface and core both 4:3.
        let (x, y, w, h) = fit_viewport(800, 600, 320, 240, 4.0 / 3.0);
        assert!(approx(x, 0.0));
        assert!(approx(y, 0.0));
        assert!(approx(w, 800.0));
        assert!(approx(h, 600.0));
    }

    #[test]
    fn zero_aspect_falls_back_to_fb_dims() {
        // PCE-style 256x239 framebuffer, no core-reported aspect; renderer
        // should treat it as 256:239 (~1.07).
        let (_x, _y, w, h) = fit_viewport(1920, 1080, 256, 239, 0.0);
        // Surface aspect (1.78) > target (1.07) → pillarbox.
        assert!(approx(h, 1080.0));
        assert!(approx(w, 1080.0 * 256.0 / 239.0));
    }
}
