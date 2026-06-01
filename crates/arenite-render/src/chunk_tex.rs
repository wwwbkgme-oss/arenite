use arenite_core::pos::{ChunkPos, CHUNK_AREA, CHUNK_SIZE};
use arenite_sim::chunk::ChunkData;

/// Per-chunk GPU texture that stores RGBA pixel data.
///
/// Each chunk maps to one `CHUNK_SIZE × CHUNK_SIZE` RGBA8 texture.
/// When ChunkData.dirty is set, we upload the CPU pixel buffer to the GPU.
pub struct ChunkTexture {
    pub pos: ChunkPos,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl ChunkTexture {
    pub fn new(
        pos: ChunkPos,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        data: &ChunkData,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: CHUNK_SIZE as u32,
            height: CHUNK_SIZE as u32,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("chunk_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chunk_bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let chunk_tex = Self {
            pos,
            texture,
            view,
            sampler,
            bind_group,
        };
        chunk_tex.upload(queue, data);
        chunk_tex
    }

    /// Upload the CPU pixel buffer to the GPU texture.
    pub fn upload(&self, queue: &wgpu::Queue, data: &ChunkData) {
        // Pack RGBA bytes from MaterialInstance colours.
        let mut rgba = Vec::with_capacity(CHUNK_AREA * 4);
        for px in &data.pixels {
            rgba.push(px.color.r);
            rgba.push(px.color.g);
            rgba.push(px.color.b);
            rgba.push(px.color.a);
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(CHUNK_SIZE as u32 * 4),
                rows_per_image: Some(CHUNK_SIZE as u32),
            },
            wgpu::Extent3d {
                width: CHUNK_SIZE as u32,
                height: CHUNK_SIZE as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}
