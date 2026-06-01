use std::marker::PhantomData;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Strongly-typed newtype ID wrapping a `u32`, parameterised by the type it identifies.
/// Prevents accidental cross-type ID mixups at compile time.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Id<T> {
    value: u32,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Construct from raw u32 — only intended for registry internals.
    #[inline]
    pub(crate) fn new(value: u32) -> Self {
        Self { value, _phantom: PhantomData }
    }

    #[inline]
    pub fn raw(self) -> u32 { self.value }
}

// Manual implementations so that `T: Clone/Copy/...` is NOT required.
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { *self }
}
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}
impl<T> Eq for Id<T> {}
impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.value.hash(state); }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.value)
    }
}
impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// String-keyed ID used in registries before numeric IDs are assigned.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct StringId(pub String);

impl StringId {
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
}

impl fmt::Display for StringId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for StringId {
    fn from(s: &str) -> Self { Self(s.into()) }
}

impl From<String> for StringId {
    fn from(s: String) -> Self { Self(s) }
}
