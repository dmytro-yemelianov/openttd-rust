import Transport.IDValidity

namespace Transport

/-!
  # Formal Verification of Budgeted Pathfinding Termination and Adjacency
  Mirrors `crates/transport-sim/src/solvers/water_path.rs`.
-/

/-- Discrete search state tracking remaining node expansion budget. -/
structure SearchState where
  budget : Nat
  expansions : Nat
  found : Bool
  deriving DecidableEq, Repr

/-- Single expansion step of a budgeted graph search. -/
def stepSearch (s : SearchState) (isTarget : Bool) : SearchState :=
  if isTarget then
    { s with found := true }
  else
    match s.budget with
    | 0 => s
    | Nat.succ b => { s with budget := b, expansions := s.expansions + 1 }

/-- Search completion predicate: either destination found or budget exhausted. -/
def isDone (s : SearchState) : Bool :=
  s.found || s.budget == 0

/-- Theorem: Search termination condition is decidable and respects budget exhaustion. -/
theorem search_done_when_budget_zero (s : SearchState) (h_zero : s.budget = 0) :
    isDone s = true := by
  dsimp [isDone]
  rw [h_zero]
  cases s.found <;> rfl

/-- Theorem: Search termination condition is true when target is reached. -/
theorem search_done_when_target_found (s : SearchState) (h_found : s.found = true) :
    isDone s = true := by
  dsimp [isDone]
  rw [h_found]
  rfl

/-- Theorem: Each expansion step strictly decrements budget when not done. -/
theorem search_budget_strictly_decreases (s : SearchState) (h_not_target : isTarget = false) (h_budget : s.budget > 0) :
    (stepSearch s isTarget).budget < s.budget := by
  dsimp [stepSearch]
  rw [h_not_target]
  cases h_b : s.budget with
  | zero =>
    omega
  | succ b =>
    dsimp
    omega

/-- Theorem: Total expansions plus remaining budget is invariant across non-target steps. -/
theorem search_budget_conservation (s : SearchState) (h_pos : s.budget > 0) :
    let s' := stepSearch s false
    s'.budget + s'.expansions = s.budget + s.expansions := by
  dsimp [stepSearch]
  cases h_b : s.budget with
  | zero =>
    omega
  | succ b =>
    dsimp
    omega

/-- Theorem: Number of expansions is strictly bounded by initial budget. -/
theorem expansions_bounded_by_initial_budget (initBudget : Nat) (s : SearchState)
    (h_inv : s.budget + s.expansions = initBudget) :
    s.expansions ≤ initBudget := by
  omega

/-- Discrete 2D grid coordinates. -/
structure GridCoord where
  x : Nat
  y : Nat
  deriving DecidableEq, Repr

/-- 4-directional cardinal adjacency predicate on tile grid. -/
def isCardinalAdjacent (a b : GridCoord) : Prop :=
  (a.x = b.x ∧ (a.y = b.y + 1 ∨ b.y = a.y + 1)) ∨
  (a.y = b.y ∧ (a.x = b.x + 1 ∨ b.x = a.x + 1))

/-- Theorem: Cardinal adjacency is symmetric. -/
theorem cardinal_adjacent_symmetric (a b : GridCoord) :
    isCardinalAdjacent a b ↔ isCardinalAdjacent b a := by
  dsimp [isCardinalAdjacent]
  omega

/-- Validity of a continuous step-by-step path on the grid. -/
def isContinuousPath : List GridCoord → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => isCardinalAdjacent a b ∧ isContinuousPath (b :: rest)

/-- Theorem: Any subpath of a valid continuous path is also continuous. -/
theorem subpath_is_continuous (a b : GridCoord) (rest : List GridCoord)
    (h_path : isContinuousPath (a :: b :: rest)) :
    isContinuousPath (b :: rest) := by
  dsimp [isContinuousPath] at h_path
  exact h_path.2

end Transport
