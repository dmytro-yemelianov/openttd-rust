# Specifications

All specifications are drafts. They distinguish upstream behavior, proposed Rust design, and new simulation behavior. They do not imply that the assessed work has already been implemented or verified at runtime.

| Specification | Contents |
|---|---|
| [SPEC-001: Translation scope and estimate](001-port-scope-and-estimate.md) | Source inventory, extraction constraints, three cumulative scope estimates, phased work, exclusions, risks and calibration gate |
| [SPEC-002: Ontology and data models](002-ontology-and-data-model.md) | Domain concepts, taxonomies, relationships, Rust ownership and identity, people and journeys, authorization boundaries, invariants and fixtures |

Read SPEC-002 first for the domain model, then SPEC-001 for delivery scope and effort.

## Evidence

- [Per-file upstream inventory](evidence/upstream-inventory.json)
- [Inventory generator](../tools/inventory.py)
- [Pinned upstream reference](../upstream.json)

## Decisions to resolve during calibration

1. Freeze the exact compatibility matrix and first ship scenario.
2. Validate crate ownership and dependencies against executable code.
3. Specify field-level schemas, ID lifecycle and snapshot versions.
4. Define operational-time mapping, authorization evidence and replay behavior with the host application.
5. Establish differential fixtures and measure performance before setting capacity targets.

Record accepted architectural decisions separately as implementation evidence becomes available; preserve the distinction between legacy parity mode and the people extension.
