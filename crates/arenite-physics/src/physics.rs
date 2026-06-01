use rapier2d::prelude::*;

/// Pixels per physics unit. The physics engine works in metres,
/// the pixel sim works in tiles.  We use 16 pixels = 1 physics metre.
pub const PHYSICS_SCALE: f32 = 16.0;

/// Wraps the full rapier2d physics pipeline.
pub struct PhysicsWorld {
    pub gravity:            Vector<f32>,
    pub integration_params: IntegrationParameters,
    pub islands:            IslandManager,
    pub broad_phase:        DefaultBroadPhase,
    pub narrow_phase:       NarrowPhase,
    pub bodies:             RigidBodySet,
    pub colliders:          ColliderSet,
    pub impulse_joints:     ImpulseJointSet,
    pub multibody_joints:   MultibodyJointSet,
    pub ccd_solver:         CCDSolver,
    pub query_pipeline:     QueryPipeline,
    pub physics_pipeline:   PhysicsPipeline,
    /// Event collector for collision callbacks.
    pub collision_events:   Vec<CollisionEvent>,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            gravity:            vector![0.0, 9.81],
            integration_params: IntegrationParameters::default(),
            islands:            IslandManager::new(),
            broad_phase:        DefaultBroadPhase::new(),
            narrow_phase:       NarrowPhase::new(),
            bodies:             RigidBodySet::new(),
            colliders:          ColliderSet::new(),
            impulse_joints:     ImpulseJointSet::new(),
            multibody_joints:   MultibodyJointSet::new(),
            ccd_solver:         CCDSolver::new(),
            query_pipeline:     QueryPipeline::new(),
            physics_pipeline:   PhysicsPipeline::new(),
            collision_events:   Vec::new(),
        }
    }

    /// Step the physics simulation by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        self.integration_params.dt = dt;
        self.collision_events.clear();

        let mut event_handler = ChannelEventCollector::new(
            crossbeam_channel::unbounded().0,
            crossbeam_channel::unbounded().0,
        );

        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &event_handler,
        );
    }

    /// Add a dynamic rectangular rigidbody.
    pub fn add_dynamic_box(
        &mut self,
        x: f32, y: f32,
        half_w: f32, half_h: f32,
        mass: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let px = x / PHYSICS_SCALE;
        let py = y / PHYSICS_SCALE;
        let hw = half_w / PHYSICS_SCALE;
        let hh = half_h / PHYSICS_SCALE;

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![px, py])
            .build();
        let body_handle = self.bodies.insert(body);

        let collider = ColliderBuilder::cuboid(hw, hh)
            .density(mass / (4.0 * hw * hh))
            .restitution(0.2)
            .friction(0.5)
            .build();
        let col_handle = self.colliders.insert_with_parent(
            collider,
            body_handle,
            &mut self.bodies,
        );

        (body_handle, col_handle)
    }

    /// Add a static rectangle (for terrain collision proxies).
    pub fn add_static_box(
        &mut self,
        x: f32, y: f32,
        half_w: f32, half_h: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::fixed()
            .translation(vector![x / PHYSICS_SCALE, y / PHYSICS_SCALE])
            .build();
        let bh = self.bodies.insert(body);

        let collider = ColliderBuilder::cuboid(
            half_w / PHYSICS_SCALE,
            half_h / PHYSICS_SCALE,
        ).build();
        let ch = self.colliders.insert_with_parent(collider, bh, &mut self.bodies);

        (bh, ch)
    }

    /// Get pixel position of a body.
    pub fn body_pixel_pos(&self, handle: RigidBodyHandle) -> Option<(f32, f32)> {
        let body = self.bodies.get(handle)?;
        let t    = body.translation();
        Some((t.x * PHYSICS_SCALE, t.y * PHYSICS_SCALE))
    }

    /// Apply a pixel-space impulse to a body.
    pub fn apply_pixel_impulse(
        &mut self,
        handle: RigidBodyHandle,
        ix: f32, iy: f32,
    ) {
        if let Some(body) = self.bodies.get_mut(handle) {
            body.apply_impulse(
                vector![ix / PHYSICS_SCALE, iy / PHYSICS_SCALE],
                true,
            );
        }
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self { Self::new() }
}
