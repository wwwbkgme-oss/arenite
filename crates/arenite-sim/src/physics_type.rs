use serde::{Deserialize, Serialize};

/// How a pixel participates in the cellular automata simulation.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum PhysicsType {
    /// Empty space — air.  The absence of matter.
    #[default]
    Air     = 0,

    /// Immovable solid (rock, dirt walls).
    Solid   = 1,

    /// Granular material: falls down, slides off piles at the angle of repose.
    Sand    = 2,

    /// Incompressible liquid: flows sideways and down to fill space.
    Liquid  = 3,

    /// Low-density gas: rises and spreads.
    Gas     = 4,

    /// Fire / plasma — spreads to flammable neighbours, self-destructs.
    Fire    = 5,

    /// Placeholder used by the physics engine to mark rigidbody-occupied pixels.
    Object  = 6,
}

impl PhysicsType {
    /// Returns `true` if this type moves during simulation.
    #[inline]
    pub fn is_dynamic(self) -> bool {
        matches!(self, Self::Sand | Self::Liquid | Self::Gas | Self::Fire)
    }

    /// Returns `true` if this type blocks movement (rigidbodies collide).
    #[inline]
    pub fn is_blocking(self) -> bool {
        matches!(self, Self::Solid | Self::Sand | Self::Object)
    }

    /// Returns `true` if this pixel allows others to displace it.
    #[inline]
    pub fn is_displaceable(self) -> bool {
        matches!(self, Self::Air | Self::Liquid | Self::Gas)
    }

    /// Returns `true` if this pixel can burn.
    #[inline]
    pub fn is_flammable(self) -> bool {
        matches!(self, Self::Gas)
    }

    /// Returns the display character for debug maps.
    pub fn char(self) -> char {
        match self {
            Self::Air    => ' ',
            Self::Solid  => '#',
            Self::Sand   => ':',
            Self::Liquid => '~',
            Self::Gas    => '\'',
            Self::Fire   => '*',
            Self::Object => 'O',
        }
    }
}


