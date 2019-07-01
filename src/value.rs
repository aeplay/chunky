use crate::{Chunk, ChunkStorage, Ident};
use std::rc::Rc;
use std::marker::PhantomData;

/// A single value stored in a chunk
pub struct Value<V> {
    chunk: Chunk,
    _marker: PhantomData<*mut V>,
}

impl<V> Value<V> {
    /// Load the value in the chunk with the given identifier, or create it using a default value
    pub fn load_or_default(ident: Ident, default: V, storage: Rc<dyn ChunkStorage>) -> Value<V> {
        let (mut chunk, created_new) = storage.load_or_create_chunk(ident, ::std::mem::size_of::<V>());

        if created_new {
            unsafe {
                ::std::ptr::write(chunk.as_mut_ptr() as *mut V, default);
            }
        }

        Value {
            chunk,
            _marker: PhantomData,
        }
    }
}

impl<V> ::std::ops::Deref for Value<V> {
    type Target = V;

    fn deref(&self) -> &V {
        unsafe { (self.chunk.as_ptr() as *const V).as_ref().unwrap() }
    }
}

impl<V> ::std::ops::DerefMut for Value<V> {
    fn deref_mut(&mut self) -> &mut V {
        unsafe { (self.chunk.as_mut_ptr() as *mut V).as_mut().unwrap() }
    }
}

impl<V> Drop for Value<V> {
    fn drop(&mut self) {
        unsafe {
            ::std::ptr::drop_in_place(self.chunk.as_mut_ptr());
        };
    }
}