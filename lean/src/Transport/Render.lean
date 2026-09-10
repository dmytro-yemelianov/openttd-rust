namespace Transport

/-!
  # Formal Verification of Presentation Layer, Depth Sorting & Interpolation Invariants
  Mirrors `crates/transport-render/src/camera.rs`, `depth.rs`, and `interpolator.rs`.
-/

/-- Linear interpolation between integer coordinates with fractional weight num / den. -/
def lerpNat (a b : Nat) (num den : Nat) : Nat :=
  if a ≤ b then
    a + ((b - a) * num) / den
  else
    b + ((a - b) * (den - num)) / den

/-- Theorem: Interpolation Lower Bound.
    When a ≤ b, lerpNat a b num den is at least a. -/
theorem lerp_lower_bound (a b num den : Nat) (h_ab : a ≤ b) :
    a ≤ lerpNat a b num den := by
  dsimp [lerpNat]
  split
  · next h_le =>
    exact Nat.le_add_right a (((b - a) * num) / den)
  · contradiction

/-- Theorem: Interpolation Upper Bound.
    When a ≤ b and num ≤ den with 0 < den, lerpNat a b num den does not exceed b. -/
theorem lerp_upper_bound (a b num den : Nat) (h_ab : a ≤ b) (h_num : num ≤ den) (h_den : 0 < den) :
    lerpNat a b num den ≤ b := by
  dsimp [lerpNat]
  split
  · next h_le =>
    have h_mul : (b - a) * num ≤ (b - a) * den := Nat.mul_le_mul_left (b - a) h_num
    have h_div : ((b - a) * num) / den ≤ ((b - a) * den) / den := Nat.div_le_div_right h_mul
    have h_cancel : ((b - a) * den) / den = b - a := Nat.mul_div_cancel (b - a) h_den
    rw [h_cancel] at h_div
    omega
  · contradiction

/-- Isometric depth function (Painter's algorithm).
    Encodes Manhattan distance along diagonal axis with elevation and layer offsets. -/
def isometricDepth (x y elev layer : Nat) : Nat :=
  (x + y) * 100 + elev * 5 + layer

/-- Theorem: Isometric Depth Monotonicity.
    For any two background/foreground positions at the same elevation and layer,
    x1 ≤ x2 and y1 ≤ y2 guarantees depth1 ≤ depth2, ensuring Painter's algorithm
    never occludes foreground tiles with background tiles. -/
theorem isometric_depth_monotonic (x1 y1 x2 y2 elev layer : Nat)
    (hx : x1 ≤ x2) (hy : y1 ≤ y2) :
    isometricDepth x1 y1 elev layer ≤ isometricDepth x2 y2 elev layer := by
  dsimp [isometricDepth]
  omega

/-- Theorem: Isometric Depth Strict Monotonicity along X.
    If a tile is strictly closer to the foreground in X, its depth is strictly greater. -/
theorem isometric_depth_strictly_monotonic_x (x1 y1 x2 y2 elev layer : Nat)
    (hx : x1 < x2) (hy : y1 ≤ y2) :
    isometricDepth x1 y1 elev layer < isometricDepth x2 y2 elev layer := by
  dsimp [isometricDepth]
  omega

/-- Theorem: Isometric Depth Strict Monotonicity along Y.
    If a tile is strictly closer to the foreground in Y, its depth is strictly greater. -/
theorem isometric_depth_strictly_monotonic_y (x1 y1 x2 y2 elev layer : Nat)
    (hx : x1 ≤ x2) (hy : y1 < y2) :
    isometricDepth x1 y1 elev layer < isometricDepth x2 y2 elev layer := by
  dsimp [isometricDepth]
  omega

/-- Orthographic world-to-screen projection along one axis. -/
def worldToScreenOrtho (x cell_w offset : Nat) : Nat :=
  x * cell_w + offset

/-- Orthographic screen-to-world inverse projection along one axis. -/
def screenToWorldOrtho (screen cell_w offset : Nat) : Nat :=
  (screen - offset) / cell_w

/-- Theorem: Orthographic Projection Bijection.
    Projecting a world tile coordinate to screen and immediately inverting it
    recovers the exact original tile coordinate without distortion. -/
theorem ortho_projection_roundtrip (x cell_w offset : Nat) (hw : 0 < cell_w) :
    screenToWorldOrtho (worldToScreenOrtho x cell_w offset) cell_w offset = x := by
  dsimp [screenToWorldOrtho, worldToScreenOrtho]
  have h_sub : (x * cell_w + offset) - offset = x * cell_w := Nat.add_sub_cancel (x * cell_w) offset
  rw [h_sub]
  exact Nat.mul_div_cancel x hw

end Transport
