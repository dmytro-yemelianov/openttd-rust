import Transport.IDValidity

namespace Transport

/-!
  # Formal Verification of Peep State and Turnstile Invariants
  Mirrors `crates/transport-people/src/peep.rs`, `turnstile.rs`, and `manifest.rs`.
-/

/-- Discrete operational state of an individual person (peep). -/
inductive PeepState where
  | atOrigin
  | walkingToStation (targetStation : StationID)
  | inStationQueue (stationId : StationID)
  | boarding (vehicleId : VehicleID)
  | aboardVehicle (vehicleId : VehicleID)
  | alighting (stationId : StationID)
  | completed
  deriving DecidableEq, Repr

/-- Invariant: A person occupies strictly one mutual state at any moment. -/
theorem peep_state_mutually_exclusive_queue_and_aboard (s : PeepState)
    (h_queue : ∃ sid, s = PeepState.inStationQueue sid)
    (h_aboard : ∃ vid, s = PeepState.aboardVehicle vid) : False := by
  rcases h_queue with ⟨sid, rfl⟩
  rcases h_aboard with ⟨vid, h_contra⟩
  nomatch h_contra

/-- Invariant: A person cannot simultaneously be completed and aboard a vehicle. -/
theorem peep_state_mutually_exclusive_completed_and_aboard (s : PeepState)
    (h_comp : s = PeepState.completed)
    (h_aboard : ∃ vid, s = PeepState.aboardVehicle vid) : False := by
  subst h_comp
  rcases h_aboard with ⟨vid, h_contra⟩
  nomatch h_contra

/-- Model of a Station Turnstile -/
structure TurnstileModel where
  queueLen : Nat
  admittedTotal : Nat
  ticketFare : Nat
  deriving DecidableEq, Repr

/-- Model of a Vehicle Manifest -/
structure ManifestModel where
  capacity : Nat
  passengerCount : Nat
  deriving DecidableEq, Repr

/-- Available seats on a vehicle. -/
def ManifestModel.availableSeats (m : ManifestModel) : Nat :=
  m.capacity - m.passengerCount

/-- Turnstile admission step: admits up to available seats from the turnstile queue. -/
def admitTurnstile (t : TurnstileModel) (m : ManifestModel) : TurnstileModel × ManifestModel × Nat :=
  let toAdmit := min m.availableSeats t.queueLen
  let t' : TurnstileModel := {
    queueLen := t.queueLen - toAdmit,
    admittedTotal := t.admittedTotal + toAdmit,
    ticketFare := t.ticketFare
  }
  let m' : ManifestModel := {
    capacity := m.capacity,
    passengerCount := m.passengerCount + toAdmit
  }
  (t', m', toAdmit)

/-- Theorem: Conservation of Turnstile Admissions.
    The increase in turnstile `admittedTotal` strictly equals the increase in vehicle `passengerCount`. -/
theorem turnstile_admission_conservation (t : TurnstileModel) (m : ManifestModel) :
    let (t', m', toAdmit) := admitTurnstile t m
    t'.admittedTotal - t.admittedTotal = m'.passengerCount - m.passengerCount ∧
    t'.admittedTotal - t.admittedTotal = toAdmit := by
  dsimp [admitTurnstile]
  omega

/-- Theorem: Passenger Capacity Bound.
    Boarding never causes vehicle occupant count to exceed maximum capacity. -/
theorem turnstile_boarding_respects_capacity (t : TurnstileModel) (m : ManifestModel)
    (h_init : m.passengerCount ≤ m.capacity) :
    let (_, m', _) := admitTurnstile t m
    m'.passengerCount ≤ m'.capacity := by
  dsimp [admitTurnstile, ManifestModel.availableSeats]
  omega

/-- Theorem: Turnstile Revenue Integrity.
    Company revenue generated from admissions is strictly equal to `toAdmit * ticketFare`. -/
def admissionRevenue (ticketFare : Nat) (toAdmit : Nat) : Nat :=
  ticketFare * toAdmit

theorem turnstile_revenue_conservation (t : TurnstileModel) (m : ManifestModel) :
    let (t', _, toAdmit) := admitTurnstile t m
    admissionRevenue t.ticketFare (t'.admittedTotal - t.admittedTotal) = admissionRevenue t.ticketFare toAdmit := by
  dsimp [admitTurnstile]
  have h : t.admittedTotal + min m.availableSeats t.queueLen - t.admittedTotal = min m.availableSeats t.queueLen := by omega
  rw [h]

end Transport
