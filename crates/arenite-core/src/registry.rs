use std::collections::HashMap;
use std::marker::PhantomData;
use crate::id::{Id, StringId};

/// A registry maps string keys to typed numeric IDs and back.
///
/// Inspired by Minecraft's and FallingSandEngine's registry patterns,
/// but generic and zero-unsafe.
pub struct Registry<T> {
    by_name: HashMap<StringId, Id<T>>,
    entries: Vec<(StringId, T)>,
    _phantom: PhantomData<T>,
}

/// A resolved reference into a `Registry<T>`.  
/// Cheap to copy; panics on access if the registry has been dropped.
pub type RegistryId<T> = Id<T>;

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            by_name:  HashMap::default(),
            entries:  Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Register a new entry.  Returns the assigned `Id<T>`.
    /// Panics if the key is already registered.
    pub fn register(&mut self, key: impl Into<StringId>, value: T) -> Id<T> {
        let key = key.into();
        assert!(!self.by_name.contains_key(&key), "Duplicate registry key: {}", key);
        let id = Id::new(self.entries.len() as u32);
        self.by_name.insert(key.clone(), id);
        self.entries.push((key, value));
        id
    }

    pub fn get_by_id(&self, id: Id<T>) -> Option<&T> {
        self.entries.get(id.raw() as usize).map(|(_, v)| v)
    }

    pub fn get_by_name(&self, key: &StringId) -> Option<&T> {
        let id = self.by_name.get(key)?;
        self.get_by_id(*id)
    }

    pub fn id_of(&self, key: &StringId) -> Option<Id<T>> {
        self.by_name.get(key).copied()
    }

    pub fn name_of(&self, id: Id<T>) -> Option<&StringId> {
        self.entries.get(id.raw() as usize).map(|(k, _)| k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &StringId, &T)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, (k, v))| (Id::new(i as u32), k, v))
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

impl<T> Default for Registry<T> {
    fn default() -> Self { Self::new() }
}

impl<T> std::ops::Index<Id<T>> for Registry<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        &self.entries[id.raw() as usize].1
    }
}
