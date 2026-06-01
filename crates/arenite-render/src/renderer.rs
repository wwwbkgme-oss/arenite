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

/// Top-level renderer — manages wgpu surface, pipelines,
/// chunk textures, and per-frame draw calls.
pub struct AreniteRenderer {
    pub device:         Arc<wgpu::Device>,
    pub queue:          Arc<wgpu::Queue>,
    surface:            wgpu::Surface<'static>,
    surface_cfg:        wgpu::SurfaceConfiguration,
    world_pipe:         WorldPipeline,
    chunk_textures:     AHashMap<ChunkPos, ChunkTexture>,
    /// Shared index buffer for the unit quad (0,0)→(1,1).
    quad_ibuf:          wgpu::Buffer,
    /// Pre-allocated vertex buffer for all chunk quads — rebuilt when chunk set changes.
    /// Layout: [Vertex2D × 4] per visible chunk, in the same order as `draw_order`.
    chunk_vbuf:         Option<wgpu::Buffer>,
    /// Chunk positions in draw order (matches `chunk_vbuf`).
    draw_order:         Vec<ChunkPos>,
    /// Set of chunk positions in the last vbuf build; used to detect changes.
    vbuf_generation:    u64,
    pub camera:         Camera2D,
    pub lighting:       LightPropagator,
    /// Sky/background clear colour — updated each frame from the current biome
    /// (see T-025).  Defaults to a neutral day-sky blue.
    pub sky_color:      [f32; 3],
}

impl AreniteRenderer {
    /// Initialise the renderer asynchronously.
    pub async fn new(
        window: Arc<winit::window::Window>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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
            width:        size.width.max(1),
            height:       size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode:   caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_cfg);

        let world_pipe = WorldPipeline::new(&device, format);

        // Shared index buffer (6 indices for one quad, reused for all chunks).
        let quad_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
            chunk_textures:  AHashMap::default(),
            quad_ibuf,
            chunk_vbuf:      None,
            draw_order:      Vec::new(),
            vbuf_generation: 0,
            camera,
            lighting:        LightPropagator::default(),
            // Linear-space sky blue default (matching the original hardcoded value).
            // Biome sky_color is applied each tick via sRGB→linear conversion.
            sky_color:       [0.22, 0.36, 0.58],
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
    /// Only re-uploads dirty chunks; creates new textures for newly loaded chunks.
    pub fn sync_chunks(&mut self, world: &SimWorld) {
        let mut changed = false;

        for (&pos, cell) in &world.chunks {
            // SAFETY: render-prep runs between sim ticks on the same thread.
            let data = unsafe { &mut *cell.get() };

            if let Some(tex) = self.chunk_textures.get(&pos) {
                if data.dirty.dirty {
                    tex.upload(&self.queue, data);
                    data.dirty = arenite_sim::chunk::DirtyRect::clean();
                }
            } else {
                let tex = ChunkTexture::new(
                    pos,
                    &self.device,
                    &self.queue,
                    &self.world_pipe.chunk_bind_layout,
                    data,
                );
                // Mark clean immediately after first upload.
                data.dirty = arenite_sim::chunk::DirtyRect::clean();
                self.chunk_textures.insert(pos, tex);
                changed = true;
            }
        }

        // Evict textures for chunks no longer in the world.
        let before = self.chunk_textures.len();
        self.chunk_textures.retain(|pos, _| world.chunks.contains_key(pos));
        if self.chunk_textures.len() != before { changed = true; }

        // Rebuild draw-order + vertex buffer when chunk set changes.
        if changed || self.chunk_vbuf.is_none() {
            self.rebuild_chunk_vbuf();
        }
    }

    /// Build one vertex buffer covering all loaded chunks (4 verts × N chunks).
    /// The buffer is rebuild only when chunks are added/removed — not every frame.
    fn rebuild_chunk_vbuf(&mut self) {
        self.draw_order = self.chunk_textures.keys().copied().collect();
        // Sort for deterministic draw order (helps avoid z-fighting on tile edges).
        self.draw_order.sort_unstable_by_key(|p| (p.y, p.x));

        if self.draw_order.is_empty() {
            self.chunk_vbuf = None;
            return;
        }

        let mut verts: Vec<Vertex2D> = Vec::with_capacity(self.draw_order.len() * 4);
        for &pos in &self.draw_order {
            let ox = (pos.x * CHUNK_SIZE) as f32;
            let oy = (pos.y * CHUNK_SIZE) as f32;
            verts.extend_from_slice(&Vertex2D::quad(ox, oy, CHUNK_SIZE as f32, CHUNK_SIZE as f32));
        }

        self.chunk_vbuf = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label:    Some("chunk_vbuf"),
                contents: bytemuck::cast_slice(&verts),
                usage:    wgpu::BufferUsages::VERTEX,
            },
        ));
        self.vbuf_generation += 1;
    }

    /// Render one frame.  Returns `Err(SurfaceError)` on swapchain problems.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view  = frame.texture.create_view(&Default::default());

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
                            r: self.sky_color[0] as f64,
                            g: self.sky_color[1] as f64,
                            b: self.sky_color[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });

            if let Some(vbuf) = &self.chunk_vbuf {
                pass.set_pipeline(&self.world_pipe.pipeline);
                pass.set_bind_group(0, &self.world_pipe.camera_bind_group, &[]);
                pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint16);
                // T-016 fix: vbuf and draw_order are stored on self and outlive the pass.
                pass.set_vertex_buffer(0, vbuf.slice(..));

                for (i, pos) in self.draw_order.iter().enumerate() {
                    if let Some(tex) = self.chunk_textures.get(pos) {
                        let base_vertex = (i * 4) as i32;
                        pass.set_bind_group(1, &tex.bind_group, &[]);
                        pass.draw_indexed(0..6, base_vertex, 0..1);
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
