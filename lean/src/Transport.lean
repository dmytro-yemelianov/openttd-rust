import Transport.IDValidity
import Transport.PersonLocation
import Transport.PathfindingBudget
import Transport.JournalIntegrity
import Transport.TraceEquivalence
import Transport.RingBuffer
import Transport.PeepEconomy
import Transport.Render
import Transport.Tools

namespace Transport

/-! ## 2. Tile Index Invariants -/

structure TileIndex where
  x : Nat
  y : Nat
  deriving DecidableEq, Repr

def validIndex (idx : TileIndex) (width height : Nat) : Prop :=
  idx.x < width ∧ idx.y < height

def toLinear (idx : TileIndex) (width : Nat) : Nat :=
  idx.y * width + idx.x

def ofLinear (linear : Nat) (width : Nat) : TileIndex :=
  ⟨linear % width, linear / width⟩

theorem ofLinear_toLinear (idx : TileIndex) (width : Nat) (hw : 0 < width) (hx : idx.x < width) :
    ofLinear (toLinear idx width) width = idx := by
  dsimp [ofLinear, toLinear]
  have h1 : (idx.y * width + idx.x) % width = idx.x := by
    rw [Nat.add_comm, Nat.add_mul_mod_self_right]
    exact Nat.mod_eq_of_lt hx
  have h2 : (idx.y * width + idx.x) / width = idx.y := by
    rw [Nat.add_comm, Nat.add_mul_div_right _ _ hw]
    rw [Nat.div_eq_of_lt hx, Nat.zero_add]
  simp [h1, h2]

theorem toLinear_ofLinear (linear : Nat) (width : Nat) :
    toLinear (ofLinear linear width) width = linear := by
  dsimp [toLinear, ofLinear]
  rw [Nat.mul_comm, Nat.div_add_mod]

/-! ## 3. Cargo Conservation Invariants -/

def splitCargo (total : Nat) (take : Nat) : Nat × Nat :=
  (total - take, take)

theorem split_conserves_cargo (total : Nat) (take : Nat) (h : take ≤ total) :
    let (rem, taken) := splitCargo total take
    rem + taken = total := by
  dsimp [splitCargo]
  exact Nat.sub_add_cancel h

def mergeCargo (c1 c2 : Nat) : Nat :=
  c1 + c2

theorem merge_conserves_cargo (c1 c2 : Nat) :
    mergeCargo c1 c2 = c1 + c2 := by
  rfl

/-! ## 4. Vehicle Order Schedule Invariants -/

inductive VehicleState where
  | idle
  | traveling
  | waiting (remainingTicks : Nat)
  | loading
  deriving DecidableEq, Repr

def nextOrderIndex (currentIdx : Nat) (orderCount : Nat) : Nat :=
  (currentIdx + 1) % orderCount

theorem nextOrderIndex_lt (currentIdx : Nat) (orderCount : Nat) (h : 0 < orderCount) :
    nextOrderIndex currentIdx orderCount < orderCount := by
  dsimp [nextOrderIndex]
  exact Nat.mod_lt _ h

/-! ## 5. Microkernel Phase Partitioning & Monotonic Ticks -/

inductive Phase where
  | ingress
  | solvers
  | drivers
  | commit
  | egress
  deriving DecidableEq, Repr

structure KernelState where
  tick : Nat
  stagedIntents : Nat
  deriving DecidableEq, Repr

def stepPhase (p : Phase) (s : KernelState) : KernelState :=
  match p with
  | Phase.ingress => s
  | Phase.solvers => s
  | Phase.drivers => s
  | Phase.commit => ⟨s.tick, 0⟩
  | Phase.egress => ⟨s.tick + 1, 0⟩

def runTickCycle (s : KernelState) : KernelState :=
  stepPhase Phase.egress (
    stepPhase Phase.commit (
      stepPhase Phase.drivers (
        stepPhase Phase.solvers (
          stepPhase Phase.ingress s))))

theorem tick_cycle_strictly_monotonic (s : KernelState) :
    s.tick < (runTickCycle s).tick := by
  dsimp [runTickCycle, stepPhase]
  exact Nat.lt_succ_self s.tick

theorem tick_cycle_deterministic (s1 s2 : KernelState) (h : s1 = s2) :
    runTickCycle s1 = runTickCycle s2 := by
  rw [h]

end Transport
