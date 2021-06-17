use crate::{ChunkStorage, Ident};
use crate::arena::{Arena, ArenaIndex};
use crate::vector::Vector;
use ::std::rc::Rc;

/// Refers to an item in a `MultiArena`
#[derive(Copy, Clone)]
pub struct MultiArenaIndex(pub usize, pub ArenaIndex);

/// Based on a collection type for fixed-size items ("Bin"), creates a collection for
/// heterogenously-sized items which will be stored in the most appropriately-sized bin.
///
/// All Bins will use children of a main chunker to create their chunks.
pub struct MultiArena {
    ident: Ident,
    typical_chunk_size: usize,
    base_size: usize,
    /// All fixed-size bins in this multi-sized collection
    ///
    /// The bin at index `i` will have item-size `base_size * 2 ^ i`
    bins: Vec<Option<Arena>>,
    used_bin_sizes: Vector<usize>,
    storage: Rc<dyn ChunkStorage>
}

impl MultiArena {
    /// Create a new `MultiArena` collection using `Arena` bins and a base size that represents
    /// the smallest expected item size (used as the item size of the smallest-sized bin)
    pub fn new(ident: Ident, typical_chunk_size: usize, base_size: usize, storage: Rc<dyn ChunkStorage>) -> Self {
        let mut multi_arena = MultiArena {
            typical_chunk_size,
            base_size,
            used_bin_sizes: Vector::<usize>::new(ident.sub("bin_sizes"), 1024, Rc::clone(&storage)),
            ident,
            bins: Vec::new(),
            storage
        };

        let n_bins = multi_arena.used_bin_sizes.len();

        for i in 0..n_bins {
            let size = *multi_arena.used_bin_sizes.at(i).unwrap();
            multi_arena.get_or_insert_bin_for_size(size);
        }

        multi_arena
    }

    fn size_rounded_multiple(&self, size: usize) -> usize {
        let size_rounded_to_base_size = (size + self.base_size - 1) / self.base_size;
        size_rounded_to_base_size.next_power_of_two()
    }

    /// Get the index of the Bin which stores items of size `size`
    pub fn size_to_index(&self, size: usize) -> usize {
        (self.size_rounded_multiple(size) as f32).log2() as usize
    }

    fn get_or_insert_bin_for_size(&mut self, size: usize) -> &mut Arena {
        let index = self.size_to_index(size);
        let size_rounded_up = self.size_rounded_multiple(size) * self.base_size;

        if index >= self.bins.len() {
            self.bins.resize_with(index + 1, Default::default)
        }

        let maybe_bin = &mut self.bins[index];

        if let Some(ref mut bin) = *maybe_bin {
            bin
        } else {
            self.used_bin_sizes.push(size_rounded_up);
            let chunk_size = ::std::cmp::max(self.typical_chunk_size, size_rounded_up);
            *maybe_bin = Some(Arena::new(
                self.ident.sub(size_rounded_up),
                chunk_size,
                size_rounded_up,
                Rc::clone(&self.storage)
            ));
            maybe_bin.as_mut().unwrap()
        }
    }

    /// Get an (untyped) pointer to the item at the given index
    pub fn at(&self, index: MultiArenaIndex) -> *const u8 {
        unsafe {
            self.bins[index.0]
                .as_ref()
                .expect("No bin at this index")
                .at(index.1)
        }
    }

    /// Get an (untyped) mutable pointer to the item at the given index
    pub fn at_mut(&mut self, index: MultiArenaIndex) -> *mut u8 {
        unsafe {
            self.bins[index.0]
                .as_mut()
                .expect("No bin at this index")
                .at_mut(index.1)
        }
    }

    /// Add an item to the end of the bin corresponding to its size
    pub fn push(&mut self, size: usize) -> (*mut u8, MultiArenaIndex) {
        let bin_index = self.size_to_index(size);
        let bin = &mut self.get_or_insert_bin_for_size(size);
        let (ptr, arena_index) = bin.push();
        (ptr, MultiArenaIndex(bin_index, arena_index))
    }

    /// Remove the item referenced by `index` from its bin by swapping with the bin's last item
    pub fn swap_remove_within_bin(&mut self, index: MultiArenaIndex) -> Option<*const u8> {
        unsafe {
            self.bins[index.0]
                .as_mut()
                .expect("No bin at this index")
                .swap_remove(index.1)
        }
    }

    /// Return indices of bins that actually contain items and their respective lengths
    pub fn populated_bin_indices_and_lens(
        &'_ self,
    ) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.bins
            .iter()
            .enumerate()
            .filter_map(|(index, maybe_bin)| maybe_bin.as_ref().map(|bin| (index, bin.len())))
    }

    /// Get the length of the bin of the given bin index
    pub fn bin_len(&self, bin_index: usize) -> usize {
        self.bins[bin_index]
            .as_ref()
            .expect("No bin at this index")
            .len()
    }
}
