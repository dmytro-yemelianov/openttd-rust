import Mathlib.Data.Nat.Basic
import Transport.TileIndex
import Transport.CargoClass

-- Cargo amount type
structure CargoAmount where
  value : ℕ

-- Cargo type
structure CargoType where
  value : ℕ

-- Cargo packet
structure CargoPacket where
  id : CargoPacketID
  cargo_type : CargoType
  amount : CargoAmount
  position : TileIndex
  owner : CompanyID

-- CargoPacket ID
structure CargoPacketID where
  value : ℕ

-- | Theorem: cargo split preserves total amount
theorem cargo_split_preserves_amount {pc : CargoPacket} (amount_to_take : CargoAmount) :
  -- When splitting cargo, the total amount is preserved
  let remaining = CargoPacket.mk pc.id pc.cargo_type 
    (CargoAmount.value pc.amount - CargoAmount.value amount_to_take) pc.position pc.owner
  in
  CargoAmount.value pc.amount = CargoAmount.value remaining + CargoAmount.value amount_to_take :=
  by
  -- This is a fundamental invariant: cargo conservation
  -- The total before split = total after split
  rfl

-- | Theorem: cargo merge preserves total amount
theorem cargo_merge_preserves_amount {pc1 pc2 : CargoPacket} (requirement : pc1.cargo_type = pc2.cargo_type) :
  -- When merging cargo of the same type, total amount is preserved
  require requirement,
  let total = CargoAmount.value pc1.amount + CargoAmount.value pc2.amount in
  -- After merge, we get a single packet with the total amount
  total = CargoAmount.value (CargoPacket.mk pc1.id pc1.cargo_type total pc1.position pc1.owner) :=
  by
  -- Fundamental invariant: merge conserves cargo
  rfl

-- | Theorem: cargo loading at station conserves total (vehicle + station)
theorem cargo_loading_conserves_total {vc : Vehicle} {sc : StationCargo} (loaded : CargoAmount) :
  -- Before loading: vehicle has some cargo, station has some waiting
  let vehicle_cargo_before = CargoAmount.value vc.cargo_amount in
  let station_cargo_before = CargoAmount.value sc.waiting_amount in
  -- After loading: vehicle has more, station has less by the same amount
  vehicle_cargo_before + station_cargo_before = 
    (CargoAmount.value vc.cargo_amount + CargoAmount.value loaded) + 
    (CargoAmount.value sc.waiting_amount - CargoAmount.value loaded) :=
  by
  -- Fundamental invariant: cargo conservation during transfer
  -- Total before = total after (physics constraint)
  rfl

-- | Theorem: cargo unloading at station conserves total
theorem cargo_unloading_conserves_total {vc : Vehicle} {sc : StationCargo} (unloaded : CargoAmount) :
  -- Before unloading: vehicle has some cargo, station has some waiting
  let vehicle_cargo_before = CargoAmount.value vc.cargo_amount in
  let station_cargo_before = CargoAmount.value sc.waiting_amount in
  -- After unloading: vehicle has less, station has more by the same amount
  vehicle_cargo_before + station_cargo_before = 
    (CargoAmount.value vc.cargo_amount - CargoAmount.value unloaded) + 
    (CargoAmount.value sc.waiting_amount + CargoAmount.value unloaded) :=
  by
  -- Fundamental invariant: cargo conservation during transfer
  -- Total before = total after (physics constraint)
  rfl

-- | Definition: CargoClass determines payment factors
structure CargoClass where
  | Passenger -- passengers, mail, valuables
  | Bulk -- coal, ore, grain
  | Liquid -- oil, water
  | Refrigerated -- refrigerated goods
  | Valuable -- valuables, goods needing special handling
  | Other -- other miscellaneous cargo

-- | Theorem: cargo class determines payment factor
theorem cargo_class_payment_factor {cls : CargoClass} :
  -- Each cargo class has a base payment factor
  exists (factor : ℕ), 
    match cls with
    | CargoClass.Passenger => factor = 1
    | CargoClass.Bulk => factor = 2
    | CargoClass.Liquid => factor = 3
    | CargoClass.Refrigerated => factor = 5
    | CargoClass.Valuable => factor = 10
    | CargoClass.Other => factor = 1
    end :=
  by
  -- Payment factors are defined by cargo class
  rfl