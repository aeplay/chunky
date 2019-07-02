use crate::{Chunk, ChunkStorage, Ident};

/// A `ChunkStorage` that allocates chunks on the heap
pub struct HeapStorage;

impl HeapStorage {
    /// Get an instance of `HeapStorage` - it doesn't have any configuration
    pub fn new() -> HeapStorage{
        HeapStorage
    }
}

impl ChunkStorage for HeapStorage {
    fn create_chunk(&self, _ident: Ident, size: usize) -> Chunk {
        //println!("Allocating chunk {} of size {}", ident.0, size);
        let mut vec = Vec::with_capacity(size);
        Chunk {
            ptr: vec.as_mut_ptr(),
            len: vec.capacity(),
            _handle_to_drop: Box::new(vec)
        }
    }

    fn load_or_create_chunk(&self, ident: Ident, size: usize) -> (Chunk, bool) {
        (self.create_chunk(ident, size), true)
    }

    fn load_chunk(&self, _ident: Ident) -> Chunk {
        panic!("can't load memory based chunks");
    }

    fn forget_chunk(&self, chunk: Chunk) {
        ::std::mem::drop(chunk);
    }
}