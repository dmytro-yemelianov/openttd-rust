use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Read result from a ring buffer consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadResult<T> {
    /// Read next available item in FIFO order.
    Ok(Arc<T>),
    /// Consumer lagged behind producer capacity. Some frames were dropped,
    /// and consumer cursor was resynchronized to the oldest retained item.
    Lagged { dropped: u64, item: Arc<T> },
    /// No new items available since last read.
    Empty,
}

/// Thread-safe, bounded, zero-copy event ring buffer.
///
/// Designed for low-latency simulation telemetry egress:
/// - Producer writes items monotonically without blocking or stalling the simulation tick.
/// - Multiple independent consumers track their own read cursors.
/// - Overrun detection: If a consumer falls behind buffer capacity, it detects
///   the exact dropped sequence gap and resynchronizes cleanly.
pub struct EventRingBuffer<T> {
    capacity: usize,
    head: AtomicU64,
    slots: Vec<RwLock<Option<Arc<T>>>>,
}

impl<T> EventRingBuffer<T> {
    /// Create a new ring buffer with the specified fixed capacity.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than zero");
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(RwLock::new(None));
        }
        Self {
            capacity,
            head: AtomicU64::new(0),
            slots,
        }
    }

    /// Get the capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Monotonic sequence number of the next item to be published.
    pub fn head(&self) -> u64 {
        self.head.load(Ordering::Acquire)
    }

    /// Publish an item into the ring buffer.
    ///
    /// Stores the item as an `Arc<T>` in the current head slot and increments
    /// the monotonic sequence counter. Overwrites oldest slot when capacity is exceeded.
    pub fn push(&self, item: T) -> u64 {
        let seq = self.head.fetch_add(1, Ordering::SeqCst);
        let slot_idx = (seq as usize) % self.capacity;

        let mut slot = self.slots[slot_idx]
            .write()
            .expect("Ring buffer slot lock poisoned");
        *slot = Some(Arc::new(item));

        seq
    }

    /// Publish an already wrapped `Arc<T>` directly to avoid re-allocations.
    pub fn push_arc(&self, item: Arc<T>) -> u64 {
        let seq = self.head.fetch_add(1, Ordering::SeqCst);
        let slot_idx = (seq as usize) % self.capacity;

        let mut slot = self.slots[slot_idx]
            .write()
            .expect("Ring buffer slot lock poisoned");
        *slot = Some(item);

        seq
    }

    /// Create an independent consumer starting from sequence 0.
    pub fn consumer(self: &Arc<Self>) -> RingConsumer<T> {
        RingConsumer {
            buffer: Arc::clone(self),
            tail: 0,
        }
    }

    /// Create a consumer starting at the current head (only reads future items).
    pub fn consumer_from_now(self: &Arc<Self>) -> RingConsumer<T> {
        let head = self.head();
        RingConsumer {
            buffer: Arc::clone(self),
            tail: head,
        }
    }
}

/// An independent consumer cursor tracking reads against an `EventRingBuffer`.
pub struct RingConsumer<T> {
    buffer: Arc<EventRingBuffer<T>>,
    tail: u64,
}

impl<T> RingConsumer<T> {
    /// Current read cursor (sequence number).
    pub fn tail(&self) -> u64 {
        self.tail
    }

    /// Attempt to read the next item from the ring buffer.
    pub fn pop(&mut self) -> ReadResult<T> {
        let head = self.buffer.head();
        let cap = self.buffer.capacity as u64;

        if self.tail >= head {
            return ReadResult::Empty;
        }

        // Check if consumer fell behind the retained window
        if head - self.tail > cap {
            let dropped = (head - self.tail) - cap;
            self.tail = head - cap;

            let slot_idx = (self.tail as usize) % self.buffer.capacity;
            let item = {
                let slot = self.buffer.slots[slot_idx]
                    .read()
                    .expect("Ring buffer slot lock poisoned");
                slot.as_ref()
                    .expect("Retained slot must contain an item")
                    .clone()
            };
            self.tail += 1;

            ReadResult::Lagged { dropped, item }
        } else {
            let slot_idx = (self.tail as usize) % self.buffer.capacity;
            let item = {
                let slot = self.buffer.slots[slot_idx]
                    .read()
                    .expect("Ring buffer slot lock poisoned");
                slot.as_ref()
                    .expect("Slot must contain item within valid window")
                    .clone()
            };
            self.tail += 1;

            ReadResult::Ok(item)
        }
    }

    /// Drain all currently available items into a vector.
    pub fn drain_available(&mut self) -> (Vec<Arc<T>>, u64) {
        let mut items = Vec::new();
        let mut total_dropped = 0;

        loop {
            match self.pop() {
                ReadResult::Ok(item) => items.push(item),
                ReadResult::Lagged { dropped, item } => {
                    total_dropped += dropped;
                    items.push(item);
                }
                ReadResult::Empty => break,
            }
        }

        (items, total_dropped)
    }
}
