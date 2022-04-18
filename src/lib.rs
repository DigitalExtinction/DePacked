use skiplist::OrderedSkipList;
use std::{fmt, marker::PhantomData, num::NonZeroU32};

/// A growable container for data.
///
/// The inserted data themselves are kept in continuous stretch of memory to
/// aid CPU memory caching. Information about unused spots / holes in the
/// allocated memory is kept in a separate ordered skip list.
///
/// The allocated memory never shrinks and is linearly proportional to peak
/// number of stored elements.
///
/// Accessing the data is very fast and with time complexity O(1). Inserting
/// and removing is slower.
///
/// Inserting has amortized time complexity O(1). Worst case single insertion
/// complexity is linear in number of stored items because the underlying
/// memory might need to be reallocated.
///
/// Removing is slowest as it has average complexity O(log(n)) in number of
/// holes in the allocated memory. Removing is fastest when number of stored
/// elements is kept close to peak number of stored elements. Actual removal
/// time is stochastic due to usage of skip list under the hood.
pub struct PackedData<T> {
    holes: OrderedSkipList<usize>,
    data: Vec<Slot<T>>,
}

impl<T> PackedData<T> {
    /// Constructs new, empty `PackedData<T>` with specific maximum expected
    /// capacity. The underlying data structures are optimized for performance
    /// for up to this capacity.
    ///
    /// Performance of item removing deteriorates if the maximum capacity is
    /// surpassed.
    ///
    /// # Arguments
    ///
    /// * `capacity` - maximum expected capacity used for optimal performance.
    pub fn with_max_capacity(capacity: usize) -> Self {
        Self {
            holes: OrderedSkipList::with_capacity(capacity),
            data: Vec::new(),
        }
    }

    /// Returns allocated capacity. This is equal to the number of items which
    /// could be stored without reallocation.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Returns number of currently stored items.
    pub fn len(&self) -> usize {
        self.data.len() - self.holes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Inserts an item to first free spot in the underlying memory and returns
    /// ID of the item.
    ///
    /// # Arguments
    ///
    /// * `item` - item to be inserted.
    pub fn insert(&mut self, item: T) -> Item<T> {
        match self.holes.pop_front() {
            Some(index) => {
                let slot = Slot::used(self.data[index].generation(), item);
                let generation = slot.generation();
                self.data[index] = slot;
                Item {
                    index,
                    generation,
                    _marker: PhantomData,
                }
            }
            None => {
                let index = self.data.len();
                let generation = unsafe { NonZeroU32::new_unchecked(1) };
                self.data.push(Slot::used(generation, item));
                Item {
                    generation: generation,
                    index,
                    _marker: PhantomData,
                }
            }
        }
    }

    /// Removes and returns an item and marks its spot as free (thus reusable
    /// for inserting).
    ///
    /// # Arguments
    ///
    /// * `item` - ID of item to be removed.
    ///
    /// # Panics
    ///
    /// Panics if such an item is not stored.
    pub fn remove(&mut self, item: Item<T>) -> T {
        let generation = self.data[item.index]
            .generation()
            .get()
            .checked_add(1)
            .unwrap_or(1);
        let mut old = Slot::empty(unsafe { NonZeroU32::new_unchecked(generation) });
        std::mem::swap(&mut old, &mut self.data[item.index]);
        self.holes.insert(item.index);
        match old {
            Slot::Used(generation, inner_item) => {
                if generation != item.generation {
                    panic!("The item is not stored!");
                }
                inner_item
            }
            _ => panic!("The item is not stored!"),
        }
    }

    /// Returns a reference to an item.
    ///
    /// # Arguments
    ///
    /// * `item` - ID of the item to be retrieved.
    ///
    /// # Panics
    ///
    /// Panics if such an item is not stored.
    pub fn get(&self, item: Item<T>) -> &T {
        match self.data.get(item.index) {
            Some(slot) => match slot {
                Slot::Used(generation, inner_item) => {
                    if *generation != item.generation {
                        panic!("The item is not stored!");
                    }
                    inner_item
                }
                Slot::Empty(_) => panic!("The item is not stored!"),
            },
            None => panic!("The item is not stored!"),
        }
    }

    /// Returns a mutable reference to an item.
    ///
    /// # Arguments
    ///
    /// * `item` - ID of the item to be retrieved.
    ///
    /// # Panics
    ///
    /// Panics if such an item is not stored.
    pub fn get_mut(&mut self, item: Item<T>) -> &mut T {
        match self.data.get_mut(item.index) {
            Some(slot) => match slot {
                Slot::Used(generation, inner_item) => {
                    if *generation != item.generation {
                        panic!("The item is not stored!");
                    }
                    inner_item
                }
                Slot::Empty(_) => panic!("The item is not stored!"),
            },
            None => panic!("The item is not stored!"),
        }
    }
}

#[derive(Eq)]
pub struct Item<T> {
    index: usize,
    generation: NonZeroU32,
    _marker: PhantomData<T>,
}

// derive(Clone, Copy) doesn't work because of this
// https://github.com/rust-lang/rust/issues/26925
impl<T> Clone for Item<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Item<T> {}

impl<T> PartialEq for Item<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> fmt::Debug for Item<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

enum Slot<T> {
    Empty(NonZeroU32),
    Used(NonZeroU32, T),
}

impl<T> Slot<T> {
    fn used(generation: NonZeroU32, item: T) -> Self {
        Self::Used(generation, item)
    }

    fn empty(generation: NonZeroU32) -> Self {
        Self::Empty(generation)
    }

    fn generation(&self) -> NonZeroU32 {
        match self {
            Self::Empty(generation) => *generation,
            Self::Used(generation, _) => *generation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_data() {
        struct Number {
            number: u32,
        }

        let num_numbers = 100;
        let mut packed = PackedData::with_max_capacity(num_numbers * 2);

        let mut items: Vec<Item<Number>> = Vec::new();
        for number in 0..num_numbers {
            items.push(packed.insert(Number {
                number: (number as u32) + 1,
            }));
        }

        assert_eq!(packed.len(), num_numbers as usize);
        let initial_capacity = packed.capacity();
        assert!(initial_capacity >= packed.len());

        for (i, &item) in items.iter().enumerate() {
            let number = packed.get(item);
            assert_eq!(number.number, (i as u32) + 1);

            let number = packed.get_mut(item);
            number.number += 2;

            let number = packed.get(item);
            assert_eq!(number.number, (i as u32) + 3);
        }

        assert_eq!(packed.len(), num_numbers as usize);
        assert!(initial_capacity >= packed.len());

        for i in 0..(num_numbers / 2) {
            let removed: Number = packed.remove(items[i * 2]);
            assert_eq!(removed.number, (i as u32) * 2 + 3);

            assert_eq!(packed.len(), num_numbers - i - 1);
            assert_eq!(packed.capacity(), initial_capacity);
        }
    }

    #[test]
    fn test_eq() {
        struct Something(u32);
        let mut packed = PackedData::with_max_capacity(2);
        let item_a = packed.insert(Something(1));
        let item_b = packed.insert(Something(1));
        assert_eq!(item_a, item_a);
        assert_ne!(item_a, item_b);
    }

    #[test]
    #[should_panic]
    fn test_remove_twice_panic() {
        struct Something(u32);
        let mut packed = PackedData::with_max_capacity(2);
        let item = packed.insert(Something(1));
        packed.remove(item);
        packed.remove(item);
    }

    #[test]
    #[should_panic]
    fn test_get_removed_panic_a() {
        struct Something(u32);
        let mut packed = PackedData::with_max_capacity(2);
        let item = packed.insert(Something(1));
        packed.remove(item);
        packed.get(item);
    }

    #[test]
    #[should_panic]
    fn test_get_removed_panic_b() {
        struct Something(u32);
        let mut packed = PackedData::with_max_capacity(2);
        packed.insert(Something(0));
        let item = packed.insert(Something(1));
        packed.insert(Something(1));
        packed.remove(item);
        packed.insert(Something(2));
        packed.get(item);
    }

    #[test]
    #[should_panic]
    fn test_get_mut_removed_panic_a() {
        struct Something(u32);
        let mut packed = PackedData::with_max_capacity(2);
        let item = packed.insert(Something(1));
        packed.remove(item);
        packed.get_mut(item);
    }

    #[test]
    #[should_panic]
    fn test_get_mut_removed_panic_b() {
        struct Something(u32);
        let mut packed = PackedData::with_max_capacity(2);
        packed.insert(Something(0));
        let item = packed.insert(Something(1));
        packed.insert(Something(2));
        packed.remove(item);
        packed.insert(Something(3));
        packed.get_mut(item);
    }

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<Slot<u64>>(), 16);
    }
}
