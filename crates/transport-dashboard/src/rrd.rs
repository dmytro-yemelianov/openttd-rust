use serde::{Deserialize, Serialize};

/// Fixed-capacity circular ring buffer with zero dynamic allocations during runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedRing<T, const N: usize> {
    buffer: Vec<T>,
    write_idx: usize,
    count: usize,
}

impl<T: Clone + Default, const N: usize> Default for FixedRing<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Default, const N: usize> FixedRing<T, N> {
    /// Construct a preallocated ring buffer of exact capacity N.
    pub fn new() -> Self {
        assert!(N > 0, "FixedRing capacity must be greater than zero");
        Self {
            buffer: vec![T::default(); N],
            write_idx: 0,
            count: 0,
        }
    }

    /// Push an element into the ring buffer, overwriting the oldest element if full.
    pub fn push(&mut self, item: T) {
        self.buffer[self.write_idx] = item;
        self.write_idx = (self.write_idx + 1) % N;
        if self.count < N {
            self.count += 1;
        }
    }

    /// Number of elements currently stored in the ring (at most N).
    pub fn len(&self) -> usize {
        self.count
    }

    /// Maximum capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        N
    }

    /// Returns true if the buffer has zero elements.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Peek the most recently pushed element.
    pub fn latest(&self) -> Option<&T> {
        if self.count == 0 {
            None
        } else {
            let idx = if self.write_idx == 0 {
                N - 1
            } else {
                self.write_idx - 1
            };
            Some(&self.buffer[idx])
        }
    }

    /// Iterator over elements in chronological order (from oldest to newest).
    pub fn iter_chronological(&self) -> impl Iterator<Item = &T> {
        let start_idx = if self.count < N {
            0
        } else {
            self.write_idx
        };

        (0..self.count).map(move |i| {
            let idx = (start_idx + i) % N;
            &self.buffer[idx]
        })
    }

    /// Export elements as a vector in chronological order.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter_chronological().cloned().collect()
    }
}
