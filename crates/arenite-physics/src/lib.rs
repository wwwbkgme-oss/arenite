// arenite-physics — Rapier2d integration
//
// Wraps rapier2d's physics pipeline so rigidbodies can interact with the
// falling-sand pixel grid.
//
// Adapted from FallingSandEngine's `fs_common/src/game/common/world/physics.rs`
// and `rigidbody.rs`:
//  - Rigidbodies contain "partially simulated pixels" of their own
//  - Before the sand sim, rigidbody AABB pixels are stamped into the world
//    grid as Object pixels (blocking sand movement and triggering impulses)
//  - After the sand sim, Object pixels are cleared

pub mod physics;
pub mod rigidbody;

pub use physics::{PhysicsWorld, PHYSICS_SCALE};
pub use rigidbody::RigidBody;
