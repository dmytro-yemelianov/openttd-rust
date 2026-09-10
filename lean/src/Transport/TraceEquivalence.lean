namespace Transport

/-!
  # Formal Verification of Oracle Trace Equivalence & Compatible State Projection
  Mirrors `crates/transport-sim/src/oracle/`.
-/

/-- Vehicle representation in the compatible projection slice. -/
structure CompatibleVehicle where
  id : Nat
  x : Nat
  y : Nat
  state : Nat
  cargoUnits : Nat
  deriving DecidableEq, Repr

/-- Compatible state slice (π-projection of the simulation state).
    Omits superset mechanics (Peeps, turnstiles, capability tokens). -/
structure CompatibleSlice where
  tick : Nat
  money : Nat
  vehicles : List CompatibleVehicle
  stationGoods : List (Nat × Nat)
  deriving DecidableEq, Repr

/-- Extended Rust simulation state containing both compatible fields and
    superset openrct2 peep extensions / turnstile queues / capability tokens. -/
structure RustExtendedState where
  tick : Nat
  money : Nat
  vehicles : List CompatibleVehicle
  stationGoods : List (Nat × Nat)
  peepCount : Nat
  peepHappinessSum : Nat
  turnstileQueueLength : Nat
  capabilityTokenCount : Nat
  deriving DecidableEq, Repr

/-- Canonical State Projection Operator π : RustExtendedState → CompatibleSlice -/
def project (s : RustExtendedState) : CompatibleSlice :=
  { tick := s.tick,
    money := s.money,
    vehicles := s.vehicles,
    stationGoods := s.stationGoods }

/-- Theorem: Extension Insensitivity.
    Modifying superset mechanics (Peeps, turnstile queues, capability tokens)
    does not alter the projected compatible slice π(s). -/
theorem project_extension_insensitivity (s : RustExtendedState)
    (newPeeps newHappiness newQueue newTokens : Nat) :
    let s' : RustExtendedState := { s with
      peepCount := newPeeps,
      peepHappinessSum := newHappiness,
      turnstileQueueLength := newQueue,
      capabilityTokenCount := newTokens
    }
    project s = project s' := by
  rfl

/-- Differential comparison operator for trace slices. -/
def slicesEqual (s1 s2 : CompatibleSlice) : Bool :=
  decide (s1 = s2)

/-- Sequential divergence detector over execution traces. -/
def findFirstDivergence : List CompatibleSlice → List CompatibleSlice → Option Nat
  | [], [] => none
  | _ :: _, [] => some 0
  | [], _ :: _ => some 0
  | s1 :: t1, s2 :: t2 =>
    if s1 = s2 then
      (findFirstDivergence t1 t2).map (· + 1)
    else
      some 0

/-- Theorem: Identical traces produce no divergence. -/
theorem divergence_of_identical_traces (t : List CompatibleSlice) :
    findFirstDivergence t t = none := by
  induction t with
  | nil => rfl
  | cons h rest ih =>
    dsimp [findFirstDivergence]
    simp [ih]

/-- Theorem: Divergence Completeness.
    If the divergence detector reports no divergence, the traces are strictly identical. -/
theorem divergence_completeness (t1 t2 : List CompatibleSlice) :
    findFirstDivergence t1 t2 = none → t1 = t2 := by
  induction t1 generalizing t2 with
  | nil =>
    intro h
    cases t2 with
    | nil => rfl
    | cons s2 rest2 =>
      dsimp [findFirstDivergence] at h
      contradiction
  | cons s1 rest1 ih =>
    intro h
    cases t2 with
    | nil =>
      dsimp [findFirstDivergence] at h
      contradiction
    | cons s2 rest2 =>
      dsimp [findFirstDivergence] at h
      split at h
      · rename_i heq
        have h_rest : (findFirstDivergence rest1 rest2).map (· + 1) = none := h
        cases h_opt : findFirstDivergence rest1 rest2 with
        | none =>
          have h_rec := ih rest2 h_opt
          rw [heq, h_rec]
        | some n =>
          rw [h_opt] at h_rest
          dsimp at h_rest
          contradiction
      · contradiction

/-- Theorem: Trace Equivalence Characterization (Soundness & Completeness).
    Two traces have zero divergence if and only if they are identical. -/
theorem divergence_iff_eq (t1 t2 : List CompatibleSlice) :
    findFirstDivergence t1 t2 = none ↔ t1 = t2 :=
  ⟨divergence_completeness t1 t2, fun h => h ▸ divergence_of_identical_traces t1⟩

/-- Theorem: First Divergence Soundness.
    If traces differ at the head, divergence is pinpointed at index 0. -/
theorem divergence_at_head (s1 s2 : CompatibleSlice) (t1 t2 : List CompatibleSlice)
    (h : s1 ≠ s2) :
    findFirstDivergence (s1 :: t1) (s2 :: t2) = some 0 := by
  dsimp [findFirstDivergence]
  simp [h]

/-- Theorem: Step-wise Bisimulation Preservation.
    If two extended states have identical projections, their projected slices match. -/
theorem bisimulation_projection_eq (s1 s2 : RustExtendedState)
    (h : project s1 = project s2) :
    slicesEqual (project s1) (project s2) = true := by
  dsimp [slicesEqual]
  simp [h]

end Transport
