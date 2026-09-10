namespace Transport

/-!
  # Formal Verification of ID Validity for openttd-rust
  Mirrors `crates/transport-types/src/id.rs` and ID exhaustion safeguards.
-/

/-- Company ID mirroring Rust `CompanyID(pub u32)` -/
structure CompanyID where
  val : Nat
  deriving DecidableEq, Repr

namespace CompanyID

/-- Sentinel value corresponding to `CompanyID::INVALID = CompanyID(u32::MAX)` -/
def invalid : CompanyID := ⟨2^32 - 1⟩

/-- Valid ID predicate: must not equal `INVALID` and within u32 range -/
def isValid (cid : CompanyID) : Bool :=
  cid.val < 2^32 - 1

theorem valid_implies_ne_invalid (cid : CompanyID) (h : cid.isValid = true) :
    cid ≠ invalid := by
  dsimp [isValid] at h
  intro heq
  cases cid
  dsimp [invalid] at heq
  injection heq with heq'
  subst heq'
  revert h
  decide

theorem invalid_is_not_valid :
    invalid.isValid = false := by
  dsimp [isValid, invalid]
  decide

end CompanyID

/-- Vehicle ID mirroring Rust `VehicleID(pub u32)` with `INVALID = u32::MAX` -/
structure VehicleID where
  val : Nat
  deriving DecidableEq, Repr

namespace VehicleID

def invalid : VehicleID := ⟨2^32 - 1⟩

def isValid (vid : VehicleID) : Bool :=
  vid.val < 2^32 - 1

end VehicleID

/-- Station ID mirroring Rust `StationID(pub u32)` with `INVALID = u32::MAX` -/
structure StationID where
  val : Nat
  deriving DecidableEq, Repr

namespace StationID

def invalid : StationID := ⟨2^32 - 1⟩

def isValid (sid : StationID) : Bool :=
  sid.val < 2^32 - 1

end StationID

/-- Person ID mirroring Rust `PersonID(pub u64)` with `INVALID = u64::MAX` -/
structure PersonID where
  val : Nat
  deriving DecidableEq, Repr

namespace PersonID

def invalid : PersonID := ⟨2^64 - 1⟩

def isValid (pid : PersonID) : Bool :=
  pid.val < 2^64 - 1

end PersonID

/--
  ID Allocation Safeguard:
  Models `Simulator::next_company_id_checked` from `crates/transport-sim/src/lib.rs`.
  Guarantees that allocating a new company ID never returns `INVALID` and never wraps around.
-/
def nextCompanyId (maxExisting : Option CompanyID) : Option CompanyID :=
  match maxExisting with
  | none => some ⟨0⟩
  | some ⟨v⟩ =>
    if v + 1 < 2^32 - 1 then
      some ⟨v + 1⟩
    else
      none

/-- Theorem: Newly allocated CompanyID is always strictly valid (never INVALID) -/
theorem nextCompanyId_is_valid (maxExisting : Option CompanyID) (newId : CompanyID) :
    nextCompanyId maxExisting = some newId → newId.isValid = true := by
  intro h
  cases maxExisting with
  | none =>
    injection h with h'
    rw [← h']
    dsimp [CompanyID.isValid]
    decide
  | some m =>
    dsimp [nextCompanyId] at h
    split at h
    · rename_i hlt
      injection h with h'
      rw [← h']
      dsimp [CompanyID.isValid]
      exact decide_eq_true hlt
    · contradiction

/-- Theorem: On exhaustion (`maxExisting >= u32::MAX - 2`), allocation returns none instead of INVALID -/
theorem nextCompanyId_exhaustion (m : CompanyID) (h : m.val ≥ 2^32 - 2) :
    nextCompanyId (some m) = none := by
  dsimp [nextCompanyId]
  split
  · rename_i hlt
    omega
  · rfl

end Transport