use core::hash::Hash;

use alloc::sync::Arc;

use crate::atom::Atom;

extern crate alloc;

#[cfg(feature = "std")]
type Lock<T> = parking_lot::RwLock<T>;

#[cfg(not(feature = "std"))]
type Lock<T> = spin::RwLock<T>;

#[cfg(not(feature = "hash"))]
type Set<T> = alloc::collections::btree_set::BTreeSet<T>;

#[cfg(feature = "hash")]
type Set<T> = hashbrown::HashSet<T>;

#[derive(Debug, Default, Clone)]
pub struct Interner {
    files: Arc<Lock<Set<Atom>>>,
}

impl Interner {
    pub fn get_or_intern<S>(&self, string: S) -> Atom
    where
        S: Into<Atom> + Hash + AsRef<str> + ?Sized,
    {
        if let Some(found) = self.files.read().get(string.as_ref()).cloned() {
            found
        } else {
            let atom = string.into();
            self.files.write().insert(atom.clone());
            atom
        }
    }

    pub fn clear(&self) {
        self.files.write().clear();
    }

    pub fn len(&self) -> usize {
        self.files.read().len()
    }

    pub fn total_bytes(&self) -> usize {
        self.files
            .read()
            .iter()
            .fold(0, |p, m| p + m.as_bytes().len())
    }
}
