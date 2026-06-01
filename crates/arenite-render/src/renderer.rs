use std::sync::Arc;
use ahash::AHashMap;
use wgpu::util::DeviceExt as _;
use arenite_core::pos::{ChunkPos, CHUNK_SIZE};
use arenite_sim::SimWorld;
use crate::camera::Camera2D;
use crate::chunk_tex::ChunkTexture;
use crate::lighting::LightPropagator;
use crate::pipeline::WorldPipeline;
use crate::vertex::Vertex2D;

/// Top-level renderer that manages the wgpu surface, pipelines,
/// chunk textures, and per-frame draw calls.
pub struct AreniteRenderer {
    pub device:   Arc<wgpu::Device>,
    pub queue:    Arc<wgpu::Queue>,
    surface:      wgpu::Surface<'static>,
    surface_cfg:  wgpu::SurfaceConfiguration,
    world_pipe:   WorldPipeline,
    chunk_textures: AHashMap<ChunkPos, ChunkTexture>,
    quad_ibuf:    wgpu::Buffer,
    pub camera:   Camera2D,
    pub lighting: LightPropagator,
}

impl AreniteRenderer {
    /// Initialise the renderer asynchronously.
    pub async fn new(
        window: Arc<winit::window::Window>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends:             wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference:       wgpu::PowerPreference::HighPerformance,
                compatible_surface:     Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("No suitable GPU adapter found"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label:             Some("arenite_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits:   wgpu::Limits::default(),
                    memory_hints:      wgpu::MemoryHints::default(),
                },
                None,
            )
            .await?;

        let device = Arc::new(device);
        let queue  = Arc::new(queue);

        let caps   = surface.get_capabilities(&adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_cfg = wgpu::SurfaceConfiguration {
            usage:        wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width:        size.width,
            height:       size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode:   caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_cfg);

        let world_pipe = WorldPipeline::new(&device, format);

        // Index buffer for the quad (shared; vertices are per-chunk).
        let quad_ibuf  = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("quad_ibuf"),
            contents: bytemuck::cast_slice(&Vertex2D::QUAD_INDICES),
            usage:    wgpu::BufferUsages::INDEX,
        });

        let camera = Camera2D::new(size.width as f32, size.height as f32);

        Ok(Self {
            device,
            queue,
            surface,
            surface_cfg,
            world_pipe,
            chunk_textures: AHashMap::default(),
            quad_ibuf,
            camera,
            lighting: LightPropagator::default(),
        })
    }

    /// Call when the window is resized.
    pub fn resize(&mut self, new_w: u32, new_h: u32) {
        if new_w == 0 || new_h == 0 { return; }
        self.surface_cfg.width  = new_w;
        self.surface_cfg.height = new_h;
        self.surface.configure(&self.device, &self.surface_cfg);
        self.camera.resize(new_w as f32, new_h as f32);
    }

    /// Sync chunk textures from the sim world.
    /// Only uploads dirty chunks.
    pub fn sync_chunks(&mut self, world: &SimWorld) {
        for (&pos, cell) in &world.chunks {
            // SAFETY: we hold the only reference during the render-prep phase,
            // which runs between sim ticks on the main thread.
            let data = unsafe { &mut *cell.get() };

            if let Some(tex) = self.chunk_textures.get(&pos) {
                if data.dirty.dirty {
                    tex.upload(&self.queue, data);
                    data.dirty = arenite_sim::chunk::DirtyRect::clean();
                }
            } else {
                // New chunk — create texture.
                let tex = ChunkTexture::new(
                    pos,
                    &self.device,
                    &self.queue,
                    &self.world_pipe.chunk_bind_layout,
                    data,
                );
                self.chunk_textures.insert(pos, tex);
            }
        }

        // Remove textures for unloaded chunks.
        self.chunk_textures.retain(|pos, _| world.chunks.contains_key(pos));
    }

    /// Render one frame.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame  = self.surface.get_current_texture()?;
        let view   = frame.texture.create_view(&Default::default());

        self.world_pipe.update_camera(&self.queue, self.camera.view_proj());

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("frame_enc") }
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05, g: 0.05, b: 0.08, a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });

            pass.set_pipeline(&self.world_pipe.pipeline);
            pass.set_bind_group(0, &self.world_pipe.camera_bind_group, &[]);
            pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint16);

            // Draw each visible chunk as a textured quad at its world-space position.
            // We write per-chunk vertex data into a temporary buffer each frame.
            // This is simple and correct; a future optimisation would use a persistent
            // instance buffer updated only when chunks load/unload (T-034).
            for (pos, tex) in &self.chunk_textures {
                let ox = (pos.x * CHUNK_SIZE) as f32;
                let oy = (pos.y * CHUNK_SIZE) as f32;
                let verts = Vertex2D::quad(ox, oy, CHUNK_SIZE as f32, CHUNK_SIZE as f32);

                // Upload per-chunk vertices then draw.
                let vbuf = self.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label:    Some("chunk_quad_vbuf"),
                        contents: bytemuck::cast_slice(&verts),
                        usage:    wgpu::BufferUsages::VERTEX,
                    },
                );
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_bind_group(1, &tex.bind_group, &[]);
                pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
