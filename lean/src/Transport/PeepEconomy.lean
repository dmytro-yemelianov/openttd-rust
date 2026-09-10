namespace Transport

/-!
  # Formal Verification of Peep Economy, Commute Journeys & Town Growth Invariants
  Mirrors `crates/transport-people/src/commute.rs`, `crates/transport-people/src/town_driver.rs`,
  and `crates/transport-world/src/town.rs`.
-/

/-- Dual-representation population state: census population in town buildings
    and active materialized commuters traveling in the simulation. -/
structure PopulationState where
  census : Nat
  active : Nat
  deriving DecidableEq, Repr

/-- Materialize a commuter from the census pool into an active network peep. -/
def materializePeep (s : PopulationState) : PopulationState :=
  if 0 < s.census then
    ⟨s.census - 1, s.active + 1⟩
  else
    s

/-- Dematerialize a commuter upon trip completion or abandonment back into census. -/
def dematerializePeep (s : PopulationState) : PopulationState :=
  if 0 < s.active then
    ⟨s.census + 1, s.active - 1⟩
  else
    s

/-- Theorem: Population Mass Conservation under Materialization.
    Materializing a peep from census into active pool conserves total population. -/
theorem population_mass_conservation_materialize (s : PopulationState) (h : 0 < s.census) :
    let s' := materializePeep s
    s'.census + s'.active = s.census + s.active := by
  dsimp [materializePeep]
  split
  · next h_pos =>
    dsimp
    omega
  · contradiction

/-- Theorem: Population Mass Conservation under Dematerialization.
    Returning an active peep back into census conserves total population. -/
theorem population_mass_conservation_dematerialize (s : PopulationState) (h : 0 < s.active) :
    let s' := dematerializePeep s
    s'.census + s'.active = s.census + s.active := by
  dsimp [dematerializePeep]
  split
  · next h_pos =>
    dsimp
    omega
  · contradiction

/-- Financial state of commuter wallet and company treasury. -/
structure FinancialState where
  peepWallet : Nat
  companyTreasury : Nat
  deriving DecidableEq, Repr

/-- Process a turnstile ticket fare transaction. -/
def payTurnstileFare (fare : Nat) (st : FinancialState) : FinancialState :=
  if fare ≤ st.peepWallet then
    ⟨st.peepWallet - fare, st.companyTreasury + fare⟩
  else
    st

/-- Theorem: Fare Currency Conservation.
    Deducting a turnstile ticket fare from a commuter's wallet and crediting it
    to the station operating company conserves total currency in the system. -/
theorem fare_currency_conservation (fare : Nat) (st : FinancialState) (h : fare ≤ st.peepWallet) :
    let st' := payTurnstileFare fare st
    st'.peepWallet + st'.companyTreasury = st.peepWallet + st.companyTreasury := by
  dsimp [payTurnstileFare]
  split
  · next h_afford =>
    dsimp
    omega
  · contradiction

/-- Active commuter admission bounded by hard capacity limit `maxPeeps`. -/
def admitCommuter (maxPeeps : Nat) (active : Nat) : Nat :=
  if active < maxPeeps then
    active + 1
  else
    active

/-- Theorem: Active Peep Pool Capacity Bound.
    Admitting a commuter never causes the active commuter count to exceed `maxPeeps`. -/
theorem active_peep_pool_bounded (maxPeeps : Nat) (active : Nat) (h : active ≤ maxPeeps) :
    admitCommuter maxPeeps active ≤ maxPeeps := by
  dsimp [admitCommuter]
  split
  · next h_lt =>
    exact h_lt
  · exact h

/-- Town fulfillment ratio and satisfaction threshold check for growth. -/
def qualifiesForGrowth (fulfilled : Nat) (demanded : Nat) (satisfaction : Nat) : Bool :=
  if demanded == 0 then
    false
  else
    -- 60% fulfillment ratio: 10 * fulfilled ≥ 6 * demanded, and satisfaction ≥ 70
    (10 * fulfilled ≥ 6 * demanded) && (satisfaction ≥ 70)

/-- Theorem: Zero Fulfilled Commutes Precludes Town Growth.
    A town with demanded commutes but zero fulfillment can never qualify for growth. -/
theorem zero_fulfilled_no_growth (demanded : Nat) (satisfaction : Nat) (h_dem : 0 < demanded) :
    qualifiesForGrowth 0 demanded satisfaction = false := by
  dsimp [qualifiesForGrowth]
  have h_ne : (demanded == 0) = false := by
    simp
    omega
  rw [h_ne]
  simp
  omega

end Transport
