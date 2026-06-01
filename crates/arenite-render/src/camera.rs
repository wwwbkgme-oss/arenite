use glam::{Mat4, Vec2, Vec3};

/// A simple 2D orthographic camera for scrolling the world view.
#[derive(Clone, Debug)]
pub struct Camera2D {
    /// World-space centre position (pixels).
    pub position:  Vec2,
    /// Zoom factor: 1.0 = 1 pixel = 1 screen pixel.
    pub zoom:      f32,
    /// Viewport size in screen pixels.
    pub viewport:  Vec2,
}

impl Camera2D {
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            zoom:     1.0,
            viewport: Vec2::new(viewport_w, viewport_h),
        }
    }

    /// Projection × View matrix for use in shaders.
    ///
    /// Renders the world centred at `self.position` with the given zoom.
    pub fn view_proj(&self) -> Mat4 {
        let half_w = self.viewport.x * 0.5 / self.zoom;
        let half_h = self.viewport.y * 0.5 / self.zoom;

        // Orthographic projection: world pixels map 1:1 to NDC at zoom=1.
        // Y is inverted (screen-space: y increases downward).
        let proj = Mat4::orthographic_rh(
            -half_w,  half_w,
             half_h, -half_h,
            -1.0, 1.0,
        );
        let view = Mat4::from_translation(Vec3::new(
            -self.position.x,
            -self.position.y,
            0.0,
        ));
        proj * view
    }

    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.position.x += dx;
        self.position.y += dy;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.125, 16.0);
    }

    /// Convert a screen pixel to a world pixel coordinate.
    pub fn screen_to_world(&self, sx: f32, sy: f32) -> Vec2 {
        let wx = (sx - self.viewport.x * 0.5) / self.zoom + self.position.x;
        let wy = (sy - self.viewport.y * 0.5) / self.zoom + self.position.y;
        Vec2::new(wx, wy)
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.viewport = Vec2::new(w, h);
    }
}
