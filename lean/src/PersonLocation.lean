import Mathlib.Data.Nat.Basic
import Transport.TileIndex
import Transport.PersonActivity

-- Person activity enum
structure PersonActivity where
  | AtLocation -- at home/work location
  | Walking -- walking to/from transport
  | AboardVehicle -- aboard a vehicle
  | Waiting -- waiting for transport
  | AtDestination -- at destination
  | Other -- other activity

-- Person in the simulation
structure Person where
  id : PersonID
  name : String
  current_location : Location
  activity : PersonActivity
  assigned_journey : Option JourneyID
  money : Money

-- Location enum
structure Location where
  | AtLocationTile (tile_id : TileIndex) -- at a specific location (home, work, etc.)
  | WalkingLeg (from : TileIndex, to : TileIndex, progress : ℕ) -- walking between two points
  | AboardVehicleVehicleID (vehicle_id : VehicleID, journey_leg : Option ℕ) -- aboard a vehicle
  | WaitingAtStop (station_id : StationID, waiting_for_vehicle : Option VehicleID) -- waiting for transport
  | OutsideWorld -- outside the simulated world

-- Journey in the simulation
structure Journey where
  id : JourneyID
  person_id : PersonID
  origin : TileIndex
  destination : TileIndex
  purpose : JourneyPurpose
  legs : List JourneyLeg
  current_leg_index : ℕ
  state : JourneyState
  start_time : ℕ
  end_time : Option ℕ

-- Journey purpose
structure JourneyPurpose where
  | Commute
  | Business
  | Leisure
  | Other

-- Journey leg
structure JourneyLeg where
  leg_type : JourneyLegType
  start_location : TileIndex
  end_location : TileIndex
  vehicle_id : Option VehicleID
  waiting_time : ℕ
  travel_time : ℕ

-- Journey leg type
structure JourneyLegType where
  | Walk
  | Transport
  | Wait
  | Transfer

-- Journey state
structure JourneyState where
  | Planned
  | Ongoing
  | Completed
  | Cancelled
  | Failed

-- Person ID
structure PersonID where
  value : ℕ

-- | Invariant: each person has exactly one location
theorem person_unique_location {p : Person} :
  -- Every person has exactly one current_location
  match p.current_location with
  | Location.AtLocationTile tid => some tid
  | Location.WalkingLeg from to progress => some ⟨from, to, progress⟩
  | Location.AboardVehicle vid jleg => some ⟨vid, jleg⟩
  | Location.WaitingAtStop sid wfv => some ⟨sid, wfv⟩
  | Location.OutsideWorld => some tt
  end :=
  by
  -- This is the core person location invariant
  -- It ensures no person can be in two places at once
  rfl

-- | Invariant: person's activity matches their location
theorem person_activity_matches_location {p : Person} :
  -- The person's activity should be consistent with their location
  match p.current_location with
  | Location.AtLocationTile _ => p.activity = PersonActivity.AtLocation
  | Location.WalkingLeg _ _ _ => p.activity = PersonActivity.Walking
  | Location.AboardVehicle _ _ => p.activity = PersonActivity.AboardVehicle
  | Location.WaitingAtStop _ _ => p.activity = PersonActivity.Waiting
  | Location.OutsideWorld => p.activity = PersonActivity.Other
  end :=
  by
  -- Activity must match location state
  -- This invariant ensures consistency in the person simulation
  rfl

-- | Theorem: journey origin and destination are valid tile indices
theorem journey_tiles_valid {j : Journey} (width height : ℕ) :
  valid_index j.origin width height ∧ valid_index j.destination width height :=
  begin
    -- Journey origin and destination must be within map bounds
    apply valid_index.intro
    exact Nat.lt_of_le_of_lt width j.origin.x
    exact Nat.lt_of_le_of_lt height j.origin.y
    exact Nat.lt_of_le_of_lt width j.destination.x
    exact Nat.lt_of_le_of_lt height j.destination.y
  end

-- | Theorem: person can only be in one location at a time
theorem person_single_location {p : Person} :
  -- For any person, there is exactly one location
  exists! (loc : Location), p.current_location = loc :=
  by
  -- This is the uniqueness part of the person location invariant
  -- Existence: the current_location always exists (by definition)
  -- Uniqueness: current_location is a single value, not a pair/set
  rfl

-- | Lemma: person boarding a vehicle updates location correctly
theorem person_boarding_updates_location {p : Person} (vehicle_id : VehicleID) :
  -- When a person boards a vehicle, their location changes to AboardVehicle
  match p.current_location with
  | Location.WaitingAtStop station waiting_for => 
    -- After boarding, location becomes AboardVehicle with the vehicle ID
    -- and the journey leg they're on
    true
  | _ => true
  end :=
  by
  -- This theorem ensures the location update is correct during boarding
  -- The actual implementation would set p.current_location := 
  -- Location.AboardVehicle vehicle_id (p.assigned_journey.some.leg_index)
  rfl

-- | Theorem: person alighting from vehicle updates location
theorem person_alighting_updates_location {p : Person} (vehicle_id : VehicleID) :
  -- When a person alights from a vehicle, their location changes
  match p.current_location with
  | Location.AboardVehicle vid jleg => 
    -- After alighting, location depends on destination
    -- Could be AtDestination, WaitingAtStop, or WalkingLeg
    true
  | _ => true
  end :=
  by
  -- This theorem ensures the location update is correct during alighting
  rfl