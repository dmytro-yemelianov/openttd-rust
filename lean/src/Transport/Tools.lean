namespace Transport

/-!
  # Formal Verification of Interaction & Tool Invariants
  Mirrors `crates/transport-tools/src/validator.rs` and `src/tool.rs`.
-/

/-- 1D absolute difference on Nat. -/
def dist1D (a b : Nat) : Nat :=
  if a ≤ b then b - a else a - b

/-- Theorem: 1D Distance Reflexivity. -/
theorem dist1D_self (a : Nat) : dist1D a a = 0 := by
  dsimp [dist1D]
  simp

/-- Theorem: 1D Distance Symmetry. -/
theorem dist1D_symm (a b : Nat) : dist1D a b = dist1D b a := by
  dsimp [dist1D]
  by_cases h : a ≤ b
  · by_cases h2 : b ≤ a
    · have : a = b := Nat.le_antisymm h h2
      subst this
      rfl
    · rw [if_pos h, if_neg h2]
  · have h2 : b ≤ a := Nat.le_of_not_le h
    rw [if_neg h, if_pos h2]

/-- Manhattan distance between two 2D grid coordinates. -/
def manhattanDist (x1 y1 x2 y2 : Nat) : Nat :=
  dist1D x1 x2 + dist1D y1 y2

/-- Theorem: Manhattan Distance Reflexivity.
    The distance from any tile to itself is strictly 0. -/
theorem manhattan_dist_self (x y : Nat) :
    manhattanDist x y x y = 0 := by
  dsimp [manhattanDist]
  rw [dist1D_self, dist1D_self]

/-- Theorem: Manhattan Distance Symmetry.
    Distance between A and B equals distance between B and A. -/
theorem manhattan_dist_symm (x1 y1 x2 y2 : Nat) :
    manhattanDist x1 y1 x2 y2 = manhattanDist x2 y2 x1 y1 := by
  dsimp [manhattanDist]
  rw [dist1D_symm x1 x2, dist1D_symm y1 y2]

/-- Catchment predicate: a target tile (tx, ty) is within catchment radius `r` of station (sx, sy). -/
def inCatchment (sx sy tx ty r : Nat) : Prop :=
  manhattanDist sx sy tx ty ≤ r

/-- Theorem: Station tile itself is always in its own catchment for any radius. -/
theorem station_in_own_catchment (sx sy r : Nat) :
    inCatchment sx sy sx sy r := by
  dsimp [inCatchment]
  rw [manhattan_dist_self]
  exact Nat.zero_le r

/-- Theorem: Catchment Monotonicity over Radius.
    If a tile is within radius r1, and r1 ≤ r2, it is within radius r2. -/
theorem catchment_monotonic_radius (sx sy tx ty r1 r2 : Nat)
    (h_in : inCatchment sx sy tx ty r1) (h_le : r1 ≤ r2) :
    inCatchment sx sy tx ty r2 := by
  dsimp [inCatchment] at *
  exact Nat.le_trans h_in h_le

/-- Station construction cost calculation: number of tiles * unit cost. -/
def stationCost (tiles unitCost : Nat) : Nat :=
  tiles * unitCost

/-- Theorem: Station Cost Additivity.
    Expanding a station by n2 tiles costs exactly the cost of the addition. -/
theorem station_cost_additive (n1 n2 unitCost : Nat) :
    stationCost (n1 + n2) unitCost = stationCost n1 unitCost + stationCost n2 unitCost := by
  dsimp [stationCost]
  exact Nat.add_mul n1 n2 unitCost

/-- Theorem: Station Cost Monotonicity.
    Larger station footprints require greater or equal capital expenditure. -/
theorem station_cost_monotonic (n1 n2 unitCost : Nat) (h : n1 ≤ n2) :
    stationCost n1 unitCost ≤ stationCost n2 unitCost := by
  dsimp [stationCost]
  exact Nat.mul_le_mul_right unitCost h

end Transport
