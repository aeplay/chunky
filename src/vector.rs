use crate::{ChunkStorage, Ident};
use crate::arena::{Arena, ArenaIndex};
use std::marker::PhantomData;

/// A vector which stores items of a known type in an `Arena`
pub struct Vector<Item: Clone> {
    arena: Arena,
    _marker: PhantomData<Item>,
}

impl<Item: Clone> Vector<Item> {
    /// Create a new chunky vector
    pub fn new(ident: Ident, chunk_size: usize, storage: ::std::rc::Rc<dyn ChunkStorage>) -> Self {
        let item_size = ::std::mem::size_of::<Item>();
        Vector {
            arena: Arena::new(ident, ::std::cmp::max(item_size, chunk_size), item_size, storage),
            _marker: PhantomData,
        }
    }

    /// Get the number of elements in the vector
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Is the chunky vector empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a reference to the item at `index`
    pub fn at(&self, index: usize) -> Option<&Item> {
        if index < self.len() {
            Some(unsafe { &*(self.arena.at(ArenaIndex(index)) as *const Item) })
        } else {
            None
        }
    }

    /// Get a mutable reference to the item at `index`
    pub fn at_mut(&mut self, index: usize) -> Option<&mut Item> {
        if index < self.len() {
            Some(unsafe { &mut *(self.arena.at(ArenaIndex(index)) as *mut Item) })
        } else {
            None
        }
    }

    /// Push an item onto the vector
    pub fn push(&mut self, item: Item) {
        unsafe {
            let item_ptr = self.arena.push().0 as *mut Item;
            *item_ptr = item;
        }
    }

    /// Remove and return the last item, if the vector wasn't empty
    pub fn pop(&mut self) -> Option<Item> {
        if self.arena.len() == 0 {
            None
        } else {
            unsafe {
                let item_ptr: *const Item =
                    self.arena.at(ArenaIndex(self.arena.len() - 1)) as *const Item;
                let item = Some(::std::ptr::read(item_ptr));
                self.arena.pop_away();
                item
            }
        }
    }
}