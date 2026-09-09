import Mathlib.Data.Nat.Basic
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Prod

namespace Transport

-- Tile index in a 2D map
structure TileIndex where
  x : ℕ
  y : ℕ

-- Validate that a tile index is within map bounds
def valid_index (idx : TileIndex) (width height : ℕ) : Bool :=
  (idx.x < width) && (idx.y < height)

-- Convert TileIndex to linear index (row-major order)
def to_linear (idx : TileIndex) (width : ℕ) : ℕ :=
  idx.y * width + idx.x

-- Convert linear index back to TileIndex (if within bounds)
def of_linear (linear : ℕ) (width height : ℕ) : Option TileIndex :=
  if width = 0 then none else
    let y := linear / width
    let x := linear % width
    if y < height then
      some ⟨x, y⟩
    else none

-- Properties of the conversion
theorem to_linear_of_linear {w h : ℕ} (hw : 0 < w) (linear : ℕ) :
    of_linear linear w h = some ⟨x, y⟩ →
    to_linear ⟨x, y⟩ w = linear :=
  sorry

theorem of_linear_to_linear {w h : ℕ} (hw : 0 < w) (idx : TileIndex) (hidx : idx.x < w ∧ idx.y < h) :
    of_linear (to_linear idx w) w h = some idx :=
  sorry

end