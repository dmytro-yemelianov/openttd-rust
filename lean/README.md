# Lean 4 Verification for OpenTTD Rust Core

This directory contains formal verification theories using the Lean 4 theorem prover to ensure mathematical correctness of key invariants in the transport simulation engine.

## Structure

- `src/Main.lean`: Entry point
- `src/TileIndex.lean`: Verified theories for tile indexing and map bounds
- (To be extended) Theories for ID systems, cargo conservation, person location, etc.

## Setup

1. Install Lean 4 (via elan: https://lean-lang.org/elan.html)
2. Ensure mathlib4 is available (will be fetched by lake)
3. Build with: `lake build`

## Verification Goals

- Tile index bounds checking and conversion correctness
- ID generation and validity properties
- Cargo conservation (split/merge operations preserve total units)
- Person location invariant (each person has exactly one location)
- Order list sharing consistency
- Movement pathfinding correctness

## Current Status

- Basic tile index theories sketched in `src/TileIndex.lean`
- Further theories to be added as engine development proceeds