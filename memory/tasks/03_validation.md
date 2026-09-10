# Task 3: Validated State and Arithmetic (`memory/tasks/03_validation.md`)

## Status: PENDING (Depends on Task 2)

## Problem Description
- Map creation and tile lookup currently panic or exhibit undefined behavior on malformed dimensions.
- Missing validation on tile bounds, key/entity-ID agreement, and ID exhaustion.
- Arithmetic on `Money`, `CargoAmount`, and `Ticks` lacks explicit overflow/underflow protection.

## Objective & Scope
1. **Dimension & Bounds Validation**: Validate map dimensions, tile counts, and allocation bounds at initialization.
2. **Explicit Adjacency**: Return typed errors or `None` for invalid tile coordinates rather than panicking.
3. **ID Safety**: ID allocators must never return `INVALID` or silently overwrite existing entities on exhaustion.
4. **Checked Arithmetic**: Implement checked arithmetic operations for currency, cargo quantities, and tick limits.

## Target Files
- [`crates/transport-world/src/map.rs`](crates/transport-world/src/map.rs)
- [`crates/transport-types/src/unit.rs`](crates/transport-types/src/unit.rs)
- [`crates/transport-types/src/error.rs`](crates/transport-types/src/error.rs)

## Acceptance Criteria & Tests
- Regression test: Reject invalid map dimensions before allocation.
- Regression test: Out-of-bounds coordinate queries return safe failure rather than panic.
- Regression test: Money underflow returns typed error.
