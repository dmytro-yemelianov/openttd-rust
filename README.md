# OpenTTD Rust core

Specifications for a reusable Rust transport simulation core, informed by OpenTTD and extended with individual people, journeys, assignments and boarding authorization.

This repository currently contains design specifications and source assessment evidence. No Rust simulation has been implemented. The initial proposed scope is a headless ship-and-crew simulation; broader OpenTTD compatibility is estimated separately.

## Specifications

- [Specification index](specs/README.md)
- [Translation scope, effort estimates and calibration plan](specs/001-port-scope-and-estimate.md)
- [Ontology, taxonomy, Rust data models and people extension](specs/002-ontology-and-data-model.md)

## Workspace layout

Keep the original reference checkout beside this repository:

```text
github/
├── OpenTTD/        # Original C++ repository
└── openttd-rust/   # Rust-core specifications and future implementation
```

The reference is a separate Git repository, not a submodule or vendored source tree. [upstream.json](upstream.json) records its origin and the exact commit used for this assessment. Future updates to the reference require an explicit baseline review.

Reproduce the source inventory from this repository's root:

```sh
python3 tools/inventory.py ../OpenTTD --output specs/evidence/upstream-inventory.json
```

The inventory records the checkout's actual HEAD. The specifications' estimates apply to the recorded assessment baseline, not automatically to later upstream versions.

## First implementation milestone

Follow the four-week calibration plan in SPEC-001: build a C++ behavioral reference, freeze fixtures and compatibility scope, implement a small deterministic Rust slice, and revise the estimates using observed results. Individual people remain an explicit extension with separate invariants and replayable authorization inputs.
