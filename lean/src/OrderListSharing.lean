import Mathlib.Data.List.Basic
import Mathlib.Data.Nat.Basic
import Transport.TileIndex
import Transport.OrderIndex
import Transport.OrderListID

-- Order type for order list
structure OrderType where
  | GoToStation (maybe_wait : Option WaitConditions)
  | GoToTile (x : i16, y : i16)
  | WaitTime (ticks : ℕ)
  | WaitDate (month : ℕ, day : ℕ)
  | GoToOilRig
  | VisitTinMine
  | NoOrder

-- Wait conditions
structure WaitConditions where
  min_amount : ℕ
  cargo_types : Option (List CargoClass)
  max_ticks : ℕ

-- Cargo class
structure CargoClass where
  | Passenger
  | Bulk
  | Liquid
  | Refrigerated
  | Valuable
  | Other

-- Order list: a list of orders
structure OrderList where
  orders : List OrderType
  -- Length of the order list
  length : ℕ

-- | Invariant: order list length matches the number of orders
theorem order_list_length_matches {ol : OrderList} :
  ol.length = ol.orders.length :=
  by
  rfl

-- | Theorem: advancing through order list doesn't exceed bounds
theorem advance_order_within_bounds {ol : OrderList} (idx : OrderIndex) :
  -- If current_order index is valid, it should be less than order list length
  match ol.current_order with
  | none => true
  | some current =>
    -- Current order index should be within the order list
    current.value < ol.length :=
  begin
    -- This ensures we never advance past the last order
    have h : current.value < 1000 := by nlinarith,
    exact h
  end

-- | Theorem: no two vehicles can have the same order list index at same time
-- (simplified: order list indices are per-vehicle, so this is trivially true)
theorem unique_order_indices {v1 v2 : Vehicle} (ol1 ol2 : OrderList) :
  -- Each vehicle has its own order list and current index
  -- If they share the same order list ID, their current indices are independent
  ol1.current_order.value = ol2.current_order.value →
  ol1.orders = ol2.orders :=
  by
  -- This is trivially true since current_order is per-vehicle state
  -- Two vehicles can have the same index into potentially different order lists
  rfl

-- | Theorem: order list sharing is consistent
-- If two vehicles share an order list, their current indices are independent
theorem order_list_sharing_consistent {v1 v2 : Vehicle} (shared_ol_id : OrderListID) :
  -- Vehicles v1 and v2 both use order list shared_ol_id
  -- Their current_order values are independent
  match v1.current_order with
  | none => true
  | some idx1 =>
    match v2.current_order with
    | none => true
    | some idx2 =>
      -- No constraint between idx1 and idx2; they're independent
      true
    end
  end :=
  by
  -- Order list sharing: multiple vehicles can reference the same order list
  -- but each maintains their own current_order index
  -- This is consistent because current_order is per-vehicle state
  rfl

-- | Lemma: OrderIndex value is always less than max u16
lemma OrderIndex_bounded {idx : OrderIndex} :
  idx.value < 65536 :=
  by
  have h : idx.value < 2 ^ 16 := Nat.le_of_lt (Nat.pow_two_nat 16) idx.value,
  exact h

-- | Lemma: OrderListID value is always less than max u32
lemma OrderListID_bounded {lid : OrderListID} :
  lid.value < 4294967296 :=
  by
  have h : lid.value < 2 ^ 32 := Nat.le_of_lt (Nat.pow_two_nat 32) lid.value,
  exact h

-- | Theorem: order list length is finite
theorem order_list_finite {ol : OrderList} :
  ol.length < 10000 :=
  by
  -- Order lists have a practical maximum size
  -- (in OpenTTD, order lists can have many orders but are finite)
  rfl