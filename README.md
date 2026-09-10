# OpenTTD Rust core

Specifications for a reusable Rust transport simulation core, informed by OpenTTD and extended with individual people, journeys, assignments and boarding authorization.

This repository contains specifications and a compiling Rust scaffold for a headless ship-and-crew simulation. Only company creation and basic map access are implemented. Vehicle orders advance incorrectly; movement is absent, and cargo transfer and person transport are placeholders. The two current tests cover map creation and tile access. C++ behavioral parity has not been established.

The simulation tick loop is distinct from the external pi coding agents used to develop this repository. Follow the [repair plan](specs/003-correctness-and-performance-plan.md) and [agent guidelines](AGENTS.md) for bounded implementation work.

Run the baseline checks from the repository root:

```sh
cargo check --workspace --locked
cargo test --workspace --locked
```

The Rust CI workflow runs these checks on pushes and pull requests to main. Existing warnings include unused imports/variables, the default workspace resolver, and the unsupported `workspace.dev-dependencies` manifest key.

## Specifications

- [Specification index](specs/README.md)
- [Translation scope, effort estimates and calibration plan](specs/001-port-scope-and-estimate.md)
- [Ontology, taxonomy, Rust data models and people extension](specs/002-ontology-and-data-model.md)

## Formal Verification

Lean files are specification sketches; some contain `sorry` placeholders. No completed formal verification of the Rust implementation is claimed. Intended invariants include:
- Tile indexing and bounds checking
- Cargo conservation properties
- Person location invariants
- ID generation and validity

Verification theories are located in the `lean/` directory.

## Workspace layout

Keep the original reference checkout beside this repository:

```text
github/
├── OpenTTD/        # Original C++ repository
└── openttd-rust/   # Rust scaffold and specifications
    └── lean/       # Lean 4 specification sketches
```

The reference is a separate Git repository, not a submodule or vendored source tree. [upstream.json](upstream.json) records its origin and the exact commit used for this assessment. Future updates to the reference require an explicit baseline review.

Reproduce the source inventory from this repository's root:

```sh
python3 tools/inventory.py ../OpenTTD --output specs/evidence/upstream-inventory.json
```

The inventory records the checkout's actual HEAD. The specifications' estimates apply to the recorded assessment baseline, not automatically to later upstream versions.

## First implementation milestone

Follow the four-week calibration plan in SPEC-001: build a C++ behavioral reference, freeze fixtures and compatibility scope, implement a small deterministic Rust slice, and revise the estimates using observed results. Individual people remain an explicit extension with separate invariants and replayable authorization inputs.
