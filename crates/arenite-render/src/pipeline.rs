use wgpu::util::DeviceExt;
use crate::vertex::{Vertex2D, CameraUniform};

/// WGSL shader source for the world chunk render pass.
/// Samples a chunk texture, applies a per-pixel light multiplier from
/// a second texture (the light map), and blends the result.
pub const WORLD_SHADER: &str = r#"
// ── Arenite Engine — World Chunk Shader ─────────────────────────────────────

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var tex:       texture_2d<f32>;
@group(1) @binding(1) var tex_samp:  sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:             vec2<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip_pos = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.uv       = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let colour = textureSample(tex, tex_samp, in.uv);
    // Discard fully transparent pixels (air).
    if colour.a < 0.01 { discard; }
    return colour;
}
"#;

/// Builds the wgpu render pipeline for chunk rendering.
pub struct WorldPipeline {
    pub pipeline:            wgpu::RenderPipeline,
    pub camera_bind_layout:  wgpu::BindGroupLayout,
    pub chunk_bind_layout:   wgpu::BindGroupLayout,
    pub camera_buffer:       wgpu::Buffer,
    pub camera_bind_group:   wgpu::BindGroup,
}

impl WorldPipeline {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("world_shader"),
            source: wgpu::ShaderSource::Wgsl(WORLD_SHADER.into()),
        });

        // Camera uniform bind group layout.
        let camera_bind_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label:   Some("camera_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty:         wgpu::BindingType::Buffer {
                        ty:                 wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:   None,
                    },
                    count: None,
                }],
            },
        );

        // Chunk texture bind group layout.
        let chunk_bind_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label:   Some("chunk_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding:    0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty:         wgpu::BindingType::Texture {
                            sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled:   false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding:    1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty:         wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count:      None,
                    },
                ],
            },
        );

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label:                Some("world_pl"),
                bind_group_layouts:   &[&camera_bind_layout, &chunk_bind_layout],
                push_constant_ranges: &[],
            },
        );

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("world_rp"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module:              &shader,
                entry_point:         Some("vs_main"),
                buffers:             &[Vertex2D::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module:              &shader,
                entry_point:         Some("fs_main"),
                targets:             &[Some(wgpu::ColorTargetState {
                    format:     surface_format,
                    blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology:   wgpu::PrimitiveTopology::TriangleList,
                cull_mode:  Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   wgpu::MultisampleState::default(),
            multiview:     None,
            cache:         None,
        });

        // Camera uniform buffer (updated every frame).
        let camera_buf_data = CameraUniform::from_matrix(glam::Mat4::IDENTITY);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("camera_buf"),
            contents: bytemuck::bytes_of(&camera_buf_data),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:  Some("camera_bg"),
            layout: &camera_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding:  0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Self { pipeline, camera_bind_layout, chunk_bind_layout, camera_buffer, camera_bind_group }
    }

    pub fn update_camera(
        &self,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
    ) {
        let uniform = CameraUniform::from_matrix(view_proj);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}
