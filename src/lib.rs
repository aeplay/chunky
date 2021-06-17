//! This crate offers an abstraction over allocating fixed-size chunks of memory
//! and different low-level collection types making use of these chunks to emulate
//! "infinite" dynamically growing storages for heterogeneously-sized items.
//!
//! Its purpose is being able to abstract storage of entity-collections
//! (such as actors in `Kay`) over both temporary heap memory and persistent
//! mmap'ed memory used for both runtime and savegames.

#![warn(missing_docs)]
//#![feature(vec_resize_default)]

mod heap_storage;
#[cfg(feature = "mmap")]
mod mmap_storage;

mod value;
mod arena;
mod vector;
mod queue;
mod multi_arena;

pub use heap_storage::HeapStorage;
#[cfg(feature = "mmap")]
pub use mmap_storage::MmapStorage;

pub use value::Value;
pub use arena::{Arena, ArenaIndex};
pub use vector::Vector;
pub use queue::Queue;
pub use multi_arena::{MultiArena, MultiArenaIndex};

/// A Chunk of general purpose memory, essentially acting as &mut [u8]
/// which can be backed by different `ChunkStorage` providers.
/// Dropping a Chunk deallocates its in-memory space
/// but keeps any persisted version of that chunk.
pub struct Chunk {
    ptr: *mut u8,
    len: usize,
    _handle_to_drop: Box<dyn std::any::Any>
}

impl ::std::ops::Deref for Chunk {
    type Target=[u8];

    fn deref(&self) -> &[u8] {
        unsafe {std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl ::std::ops::DerefMut for Chunk {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

/// A provider of backing storage for `Chunks`
pub trait ChunkStorage {
    /// Create a chunk with a given identifier
    fn create_chunk(&self, ident: Ident, size: usize) -> Chunk;
    /// Load a chunk with a given identifier, or create it if it doesn't exist
    /// returns (chunk, true) if the chunk was created new rather than loaded
    fn load_or_create_chunk(&self, ident: Ident, size: usize) -> (Chunk, bool);
    /// Load a chunk with a given identifier, assumes it exists
    fn load_chunk(&self, ident: Ident) -> Chunk;
    /// Deallocate a chunk and delete any persisted representation of it
    /// (unlike Drop, which only unloads a chunk)
    fn forget_chunk(&self, chunk: Chunk);
}

/// Identifies a chunk or chunk group uniquely
#[derive(Clone)]
pub struct Ident(pub String);

impl Ident {
    /// Create a sub-identifier within a group
    pub fn sub<T: ::std::fmt::Display>(&self, suffix: T) -> Ident {
        Ident(format!("{}_{}", self.0, suffix))
    }
}

impl<T: ::std::fmt::Display> From<T> for Ident {
    fn from(source: T) -> Self {
        Ident(format!("{}", source))
    }
}