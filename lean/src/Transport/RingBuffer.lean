namespace Transport

/-!
  # Formal Verification of Event Ring Buffer Pointer Arithmetic & Loss Detection
  Mirrors `crates/transport-api/src/ring.rs`.
-/

/-- Ring buffer configuration with strictly positive capacity. -/
structure RingConfig where
  capacity : Nat
  h_cap : 0 < capacity
  deriving DecidableEq, Repr

/-- Producer and consumer cursor state in monotonic sequence space. -/
structure CursorState where
  head : Nat
  tail : Nat
  deriving DecidableEq, Repr

/-- Read status of a consumer observing the ring buffer. -/
inductive ReadOutcome where
  | empty
  | ok (unread : Nat)
  | lagged (dropped : Nat) (retained : Nat)
  deriving DecidableEq, Repr

/-- Consumer cursor evaluation function. -/
def evaluateRead (cfg : RingConfig) (st : CursorState) : ReadOutcome :=
  if st.tail ≥ st.head then
    ReadOutcome.empty
  else
    let diff := st.head - st.tail
    if diff ≤ cfg.capacity then
      ReadOutcome.ok diff
    else
      ReadOutcome.lagged (diff - cfg.capacity) cfg.capacity

/-- Resynchronize a lagged consumer cursor to the oldest valid retained item. -/
def resyncTail (cfg : RingConfig) (head : Nat) : Nat :=
  head - cfg.capacity

/-- Theorem: Loss Detection Conservation.
    When a consumer lags (head - tail > capacity), the number of reported dropped
    items plus the retained capacity exactly equals the sequence difference (head - tail). -/
theorem loss_detection_soundness (cfg : RingConfig) (st : CursorState)
    (h_lag : cfg.capacity < st.head - st.tail) :
    let diff := st.head - st.tail
    (diff - cfg.capacity) + cfg.capacity = diff := by
  dsimp
  have h_le : cfg.capacity ≤ st.head - st.tail := Nat.le_of_lt h_lag
  exact Nat.sub_add_cancel h_le

/-- Theorem: Resynchronized Cursor Window Invariant.
    After resynchronizing a lagged consumer, the distance from the resynchronized
    tail to the head is exactly equal to capacity, ensuring all reads fall in the valid window. -/
theorem resync_tail_valid (cfg : RingConfig) (head : Nat) (h_head : cfg.capacity ≤ head) :
    head - (resyncTail cfg head) = cfg.capacity := by
  dsimp [resyncTail]
  exact Nat.sub_sub_self h_head

/-- Theorem: Operational Loss Conservation.
    Whenever evaluateRead returns a lagged outcome, the reported dropped items
    and retained capacity sum exactly to the sequence difference between head and tail. -/
theorem evaluate_lagged_conservation (cfg : RingConfig) (st : CursorState) (d r : Nat)
    (h : evaluateRead cfg st = ReadOutcome.lagged d r) :
    d + r = st.head - st.tail := by
  dsimp [evaluateRead] at h
  split at h
  · contradiction
  · split at h
    · contradiction
    · injection h with hd hr
      rw [← hd, ← hr]
      have h_le : cfg.capacity ≤ st.head - st.tail := by omega
      exact Nat.sub_add_cancel h_le

/-- Theorem: Resync Strictly Advances Lagged Tail.
    When a consumer lags, resynchronizing the tail strictly advances it past the old tail. -/
theorem resync_tail_advances_monotone (cfg : RingConfig) (st : CursorState)
    (h_lag : cfg.capacity < st.head - st.tail) :
    st.tail < resyncTail cfg st.head := by
  dsimp [resyncTail]
  omega

/-- Theorem: Resync Never Exceeds Head.
    Resynchronizing the tail with positive capacity never places the tail ahead of head. -/
theorem resync_tail_le_head (cfg : RingConfig) (head : Nat) :
    resyncTail cfg head ≤ head := by
  dsimp [resyncTail]
  exact Nat.sub_le head cfg.capacity

end Transport
