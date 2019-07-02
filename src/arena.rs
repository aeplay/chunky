use crate::{Chunk, ChunkStorage, Ident};
use crate::value::Value;
use std::rc::Rc;

/// Refers to an item within an `Arena`
#[derive(Copy, Clone)]
pub struct ArenaIndex(pub usize);

/// Stores items of a fixed (max) size consecutively in a collection of chunks
pub struct Arena {
    ident: Ident,
    chunks: Vec<Chunk>,
    chunk_size: usize,
    item_size: usize,
    len: Value<usize>,
    storage: Rc<dyn ChunkStorage>
}

impl Arena {
    /// Create a new arena given a chunk group identifier, chunk size and (max) item size
    pub fn new(ident: Ident, chunk_size: usize, item_size: usize, storage: Rc<dyn ChunkStorage>) -> Arena {
        assert!(chunk_size >= item_size);

        let len = Value::<usize>::load_or_default(ident.sub("len"), 0, Rc::clone(&storage));
        let mut chunks = Vec::new();

        let mut item_offset = 0;

        while item_offset < *len {
            chunks.push(storage.load_chunk(ident.sub(item_offset)));
            item_offset += chunk_size / item_size;
        }

        Arena {
            ident,
            chunks,
            chunk_size,
            item_size,
            len,
            storage
        }
    }

    fn items_per_chunk(&self) -> usize {
        self.chunk_size / self.item_size
    }

    /// Number of elements in the collection
    pub fn len(&self) -> usize {
        *self.len
    }

    /// Is the collection empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Allocate space for a new item, returns a pointer to where the new item
    /// can be written to and the index that the new item will have.
    ///
    /// This is handled like this so items of heterogeneous types or sizes less
    /// than the fixed item size can be added to the collection.
    pub fn push(&mut self) -> (*mut u8, ArenaIndex) {
        // Make sure the item can fit in the current chunk
        if (*self.len + 1) > self.chunks.len() * self.items_per_chunk() {
            // If not, create a new chunk
            self.chunks
                .push(self.storage.create_chunk(self.ident.sub(*self.len), self.chunk_size));
        }
        let offset = (*self.len % self.items_per_chunk()) * self.item_size;
        let index = ArenaIndex(*self.len);
        *self.len += 1;
        unsafe {
            (
                self.chunks.last_mut().unwrap().as_mut_ptr().offset(offset as isize),
                index,
            )
        }
    }

    /// Remove the last item from the end
    pub fn pop_away(&mut self) {
        *self.len -= 1;
        // If possible, remove the last chunk as well
        if *self.len % self.items_per_chunk() == 0 {
            self.storage.forget_chunk(self.chunks.pop().expect("should have chunk left"));
        }
    }

    /// Remove the item at index, by swapping it with the last item
    /// and then popping, returning the swapped in item (unless empty).
    ///
    /// This is a O(1) way of removing an item if the order of items doesn't matter.
    pub unsafe fn swap_remove(&mut self, index: ArenaIndex) -> Option<*const u8> {
        assert!(*self.len > 0);
        let last_index = *self.len - 1;
        if last_index == index.0 {
            // if swapping last item
            self.pop_away();
            None
        } else {
            let last = self.at(ArenaIndex(*self.len - 1));
            let at_index = self.at_mut(index);
            ::std::ptr::copy_nonoverlapping(last, at_index, self.item_size);
            self.pop_away();
            Some(self.at(index))
        }
    }

    /// Get a pointer to the item at `index`
    pub unsafe fn at(&self, index: ArenaIndex) -> *const u8 {
        self.chunks[index.0 / self.items_per_chunk()]
            .as_ptr()
            .offset(((index.0 % self.items_per_chunk()) * self.item_size) as isize)
    }

    /// Get a mutable pointer to the item at `index`
    pub unsafe fn at_mut(&mut self, index: ArenaIndex) -> *mut u8 {
        let items_per_chunk = self.items_per_chunk();
        self.chunks[index.0 / items_per_chunk]
            .as_mut_ptr()
            .offset(((index.0 % items_per_chunk) * self.item_size) as isize)
    }
}