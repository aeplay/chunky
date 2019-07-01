use crate::{Chunk, ChunkStorage, Ident};
use crate::value::Value;
use std::rc::Rc;

/// A FIFO queue which stores heterogeneously sized items
pub struct Queue {
    ident: Ident,
    typical_chunk_size: usize,
    chunks: Vec<Chunk>,
    first_chunk_at: Value<usize>,
    last_chunk_at: Value<usize>,
    read_at: Value<usize>,
    write_at: Value<usize>,
    len: Value<usize>,
    chunks_to_drop: Vec<Chunk>,
    storage: Rc<dyn ChunkStorage>
}

// TODO invent a container struct with NonZero instead
enum NextItemRef {
    SameChunk(usize),
    NextChunk,
}

impl Queue {
    /// Create a new queue
    pub fn new(ident: &Ident, typical_chunk_size: usize, storage: Rc<dyn ChunkStorage>) -> Self {
        let mut queue = Queue {
            first_chunk_at: Value::load_or_default(ident.sub("first_chunk"), 0, Rc::clone(&storage)),
            last_chunk_at: Value::load_or_default(ident.sub("last_chunk"), 0, Rc::clone(&storage)),
            read_at: Value::load_or_default(ident.sub("read"), 0, Rc::clone(&storage)),
            write_at: Value::load_or_default(ident.sub("write"), 0, Rc::clone(&storage)),
            len: Value::load_or_default(ident.sub("len"), 0, Rc::clone(&storage)),
            ident: ident.clone(),
            typical_chunk_size,
            chunks: Vec::new(),
            chunks_to_drop: Vec::new(),
            storage: storage
        };

        // if the persisted last_chunk_at is > 0, persisted chunks need to be loaded
        if *queue.last_chunk_at > 0 {
            let mut chunk_offset = *queue.first_chunk_at;
            while chunk_offset <= *queue.last_chunk_at {
                let chunk = queue.storage.load_chunk(ident.sub(chunk_offset));
                chunk_offset += chunk.len();
                queue.chunks.push(chunk);
            }
        }

        queue
    }

    /// Number of items in the queue
    pub fn len(&self) -> usize {
        *self.len
    }

    /// Is the queue empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Enqueue an item of a given size. Returns a pointer that the item can be written to.
    ///
    /// This is handled like this so items of heterogeneous types can be enqueued.
    // TODO: return done_guard to mark as concurrently readable
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn enqueue(&mut self, size: usize) -> *mut u8 {
        enum EnqueueResult {
            Success(*mut u8),
            RetryInNewChunkOfSize(usize),
        };

        let result = {
            let ref_size = ::std::mem::size_of::<NextItemRef>();
            let offset = *self.write_at - *self.last_chunk_at;
            let chunk = self.chunks.last_mut().expect("should always have a chunk");
            let entry_ptr = chunk.as_mut_ptr().offset(offset as isize);

            // one more next item ref needs to fit afterwards,
            // even if it will just be a jump marker!
            let min_space = ref_size + size + ref_size;

            if let Some(chunk) = self.chunks.last_mut() {
                if offset + min_space <= chunk.len() {
                    // store the item size as a header
                    *(entry_ptr as *mut NextItemRef) = NextItemRef::SameChunk(ref_size + size);
                    let payload_ptr = entry_ptr.offset(ref_size as isize);
                    *self.write_at += ref_size + size;
                    *self.len += 1;
                    // return the pointer to where the item can be written
                    EnqueueResult::Success(payload_ptr)
                } else {
                    //println!("Not enough space. Offset: {}, Min Space: {},
                    //          Chunk size: {}", offset, min_space, chunk.size);
                    // store a jump marker instead of item size
                    *(entry_ptr as *mut NextItemRef) = NextItemRef::NextChunk;
                    let new_chunk_size = ::std::cmp::max(self.typical_chunk_size, min_space);
                    // retry at the beginning of a new chunk
                    *self.last_chunk_at += chunk.len();
                    *self.write_at = *self.last_chunk_at;
                    EnqueueResult::RetryInNewChunkOfSize(new_chunk_size)
                }
            } else {
                // create first chunk
                let new_chunk_size = ::std::cmp::max(self.typical_chunk_size, min_space);
                EnqueueResult::RetryInNewChunkOfSize(new_chunk_size)
            }

        };

        match result {
            EnqueueResult::Success(payload_ptr) => payload_ptr,
            EnqueueResult::RetryInNewChunkOfSize(new_chunk_size) => {
                self.chunks.push(self.storage.create_chunk(
                    self.ident.sub(*self.last_chunk_at),
                    new_chunk_size,
                ));
                self.enqueue(size)
            }
        }
    }

    /// Dequeue an item. Returns a pointer to the item in the queue, unless the queue is empty.
    // TODO: return done_guard to mark as droppable
    pub unsafe fn dequeue(&mut self) -> Option<*const u8> {
        enum DequeueResult {
            Empty,
            Success(*const u8),
            RetryInNextChunk,
        };

        let result = if *self.read_at == *self.write_at {
            DequeueResult::Empty
        } else {
            let offset = *self.read_at - *self.first_chunk_at;
            let chunk = &mut self.chunks[0];
            let entry_ptr = chunk.as_mut_ptr().offset(offset as isize);

            #[allow(clippy::cast_ptr_alignment)]
            match *(entry_ptr as *mut NextItemRef) {
                NextItemRef::NextChunk => {
                    *self.first_chunk_at += chunk.len();
                    *self.read_at = *self.first_chunk_at;
                    DequeueResult::RetryInNextChunk
                }
                NextItemRef::SameChunk(total_size) => {
                    let payload_ptr = entry_ptr.offset(::std::mem::size_of::<NextItemRef>() as isize);
                    *self.read_at += total_size;
                    *self.len -= 1;
                    DequeueResult::Success(payload_ptr)
                }
            }
        };

        match result {
            DequeueResult::Empty => None,
            DequeueResult::Success(payload_ptr) => Some(payload_ptr),
            DequeueResult::RetryInNextChunk => {
                self.chunks_to_drop.push(self.chunks.remove(0));
                self.dequeue()
            }
        }
    }

    /// Delete chunks which have already been read
    pub unsafe fn drop_old_chunks(&mut self) {
        for chunk in self.chunks_to_drop.drain(..) {
            self.storage.forget_chunk(chunk);
        }
    }
}