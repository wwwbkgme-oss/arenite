use bytemuck::{Pod, Zeroable};

/// A 2D vertex with position + UV texture coordinate.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex2D {
    /// Clip-space position (world pixels; transformed by camera view_proj).
    pub position: [f32; 2],
    /// Normalised UV coordinate in [0, 1].
    pub uv:       [f32; 2],
}

impl Vertex2D {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex2D>() as wgpu::BufferAddress,
        step_mode:    wgpu::VertexStepMode::Vertex,
        attributes:   &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
    };

    /// Build a unit quad [0,0 .. w,h] with UV [0,0..1,1].
    pub fn quad(x: f32, y: f32, w: f32, h: f32) -> [Self; 4] {
        [
            Self { position: [x,     y    ], uv: [0.0, 0.0] },
            Self { position: [x + w, y    ], uv: [1.0, 0.0] },
            Self { position: [x + w, y + h], uv: [1.0, 1.0] },
            Self { position: [x,     y + h], uv: [0.0, 1.0] },
        ]
    }

    /// Indices for two triangles covering a quad (CCW winding).
    pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
}

/// Camera uniform block pushed to the GPU every frame.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn from_matrix(m: glam::Mat4) -> Self {
        Self { view_proj: m.to_cols_array_2d() }
    }
}
