use crate::{Chunk, ChunkStorage, Ident};
use std::fs::{OpenOptions, File};
use std::path::{Path, PathBuf};
use memmap::MmapMut;

/// A `ChunkStorage` that allocates chunks by mmapping files
pub struct MmapStorage {
    pub directory: PathBuf
}

pub struct MmapStorageHandle(MmapMut, File, Ident);

impl Drop for MmapStorageHandle {
    fn drop(&mut self) {
        self.0.flush().expect(format!("Couldn't flush file {}", &((self.2).0)).as_str());
    }
}

impl MmapStorage {
    fn chunk_from_file(file: File, file_path: &Path, ident: Ident) -> Chunk {
        let mut handle = MmapStorageHandle(
            unsafe { MmapMut::map_mut(&file).expect(format!("Can't mmap file {}", file_path.to_string_lossy()).as_str())},
            file,
            ident
        );

        Chunk {
            ptr: handle.0.as_mut_ptr(),
            len: handle.0.len(),
            _handle_to_drop: Box::new(handle)
        }
    }
}

impl ChunkStorage for MmapStorage {
    fn create_chunk(&self, ident: Ident, size: usize) -> Chunk {
        let file_path = self.directory.join(&ident.0);
        let file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create_new(true)
                            .open(&file_path).expect(format!("Can't create file {}", file_path.to_string_lossy()).as_str());
        file.set_len(size as u64).expect(format!("Can't grow file {}", file_path.to_string_lossy()).as_str());

        Self::chunk_from_file(file, &file_path, ident)
    }

    fn load_or_create_chunk(&self, ident: Ident, size: usize) -> (Chunk, bool) {
        let file_path = self.directory.join(&ident.0);
        let existed = ::std::fs::metadata(&file_path).is_ok();

        let file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(&file_path).expect(format!("Can't load or create file {}", file_path.to_string_lossy()).as_str());
        if !existed {
            file.set_len(size as u64).expect(format!("Can't grow file {}", file_path.to_string_lossy()).as_str());
        }

        (Self::chunk_from_file(file, &file_path, ident), existed)
    }

    fn load_chunk(&self, ident: Ident) -> Chunk {
        let file_path = self.directory.join(&ident.0);
        let file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open(&file_path).expect(format!("Can't load file {}", file_path.to_string_lossy()).as_str());

        Self::chunk_from_file(file, &file_path, ident)
    }

    /// Deallocate a chunk and delete any persisted representation of it
    /// (unlike Drop, which only unloads a chunk)
    fn forget_chunk(&self, chunk: Chunk) {
        let handle = chunk._handle_to_drop.downcast::<MmapStorageHandle>().expect("MmapStorage got handed a foreign chunk.");
        let ident = handle.2.clone();
        let file_path = self.directory.join(&ident.0);
        std::mem::drop(handle);
        ::std::fs::remove_file(&file_path).expect(format!("Couldn't remove file {}", file_path.to_string_lossy()).as_str());
    }
}