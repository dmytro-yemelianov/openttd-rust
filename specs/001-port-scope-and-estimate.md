# SPEC-001: Rust translation scope and estimate

Status: Draft. Source assessment and proposed delivery scope; not an implementation commitment.

**Recommendation:** begin with a headless ship-and-crew subset, keeping upstream C++ as the behavioral reference. A clean Rust core is feasible, but the original core is not already extracted. Full OpenTTD replacement is a separate, multi-year compatibility project.

Prepared 2026-09-09. This is a source-based rough-order estimate, not measured port throughput or a delivery commitment. No Rust port was implemented and neither application was built or run during this assessment.

## Checkout and evidence

- Full original repository cloned to `/Users/dmytro/github/OpenTTD` (not a shallow clone).
- Origin: `https://github.com/OpenTTD/OpenTTD.git`.
- Assessed commit: [`18fc940ebf4a837d8f510050774ec20a2acab63a`](https://github.com/OpenTTD/OpenTTD/tree/18fc940ebf4a837d8f510050774ec20a2acab63a).
- No tracked source changes were made to upstream. Untracked macOS `.DS_Store` metadata appeared during inspection and was left alone.
- Indexed as graph project `OpenTTD`: 32,893 nodes and 164,534 edges. Nine files were quarantined by the indexer; graph results are incomplete and overloaded/template call edges require source confirmation. The graph's C/C++ language labels are not used for size measurement.
- [Domain model](002-ontology-and-data-model.md): ontology, taxonomy, relationships, Rust ownership proposal, person extension and invariants.
- [Inventory](evidence/upstream-inventory.json), reproduced by [inventory.py](../tools/inventory.py): tracked-file counts with explicit exclusions and classification rules.

Reproduce the inventory from the repository root:

```sh
python3 tools/inventory.py ../OpenTTD --output specs/evidence/upstream-inventory.json
```

This command measures the checkout's current HEAD and records that commit. Return to the assessed commit in an isolated worktree if upstream has since moved.

## What was measured

| Inventory category | Files | Physical lines |
|---|---:|---:|
| Remaining application/domain files | 401 | 142,747 |
| Presentation by filename/directory heuristic | 196 | 74,424 |
| Scripting adapters and APIs | 198 | 35,592 |
| NewGRF files | 107 | 28,195 |
| Networking | 75 | 23,584 |
| Save/load | 78 | 20,995 |
| Tables | 41 | 20,645 |
| Platform and drivers | 86 | 19,718 |
| Utilities and clocks | 61 | 10,277 |
| Routing / link graph | 47 | 9,635 |
| Unit-test sources/headers | 21 | 3,114 |
| Generators | 4 | 1,907 |
| **First-party total** | **1,315** | **390,833** |
| Vendored sources | 64 | 88,792 |
| **All measured source/header files** | **1,379** | **479,625** |

Physical lines include comments and blanks. Extensions are C/C++/Objective-C sources and headers, not all repository files. This is not executable SLOC, translated-line volume, or a core dependency closure. Categories are exclusive but approximate: GUI files can also appear in other subsystem buckets, and data tables need not be hand-translated. No effort estimate is calculated from these line counts.

There are four tracked regression scenario saves and a unit-test target. This is useful existing infrastructure, but its existence does not establish sufficient behavioral coverage for a port. No coverage percentage was measured.

## Why this is more than syntax translation

1. **There is no supported core-library boundary.** CMake builds an `openttd_lib` object target for the application and tests; source lists include GUI/application files. [CMakeLists.txt](https://github.com/OpenTTD/OpenTTD/blob/18fc940ebf4a837d8f510050774ec20a2acab63a/CMakeLists.txt#L244).
2. **State is distributed globally.** Map arrays, entity pools, clocks, current company and settings need an explicit world owner in a reusable Rust library. [Map storage](https://github.com/OpenTTD/OpenTTD/blob/18fc940ebf4a837d8f510050774ec20a2acab63a/src/map_func.h), [pool types](https://github.com/OpenTTD/OpenTTD/blob/18fc940ebf4a837d8f510050774ec20a2acab63a/src/core/pool_type.hpp).
3. **Tick ordering is observable behavior.** Calendar/economy/tick timers, vehicle/tile processing, scripts and notifications are ordered in one application loop. Timer priorities explicitly protect deterministic random-number use. [Loop](https://github.com/OpenTTD/OpenTTD/blob/18fc940ebf4a837d8f510050774ec20a2acab63a/src/openttd.cpp#L1207), [timer contract](https://github.com/OpenTTD/OpenTTD/blob/18fc940ebf4a837d8f510050774ec20a2acab63a/src/timer/timer_game_common.h).
4. **Data layout hides semantics.** Tile bitfields, pool IDs, cached values, multi-part vehicles and overloaded destination IDs cannot be mechanically replaced by independent Rust objects without behavioral decisions.
5. **Compatibility reconstructs history.** Save/load includes afterload migrations and reference repair. NewGRF affects behavior through callbacks and persistent storage. Parsing files is only the first part of supporting them.
6. **People are new domain behavior.** Cargo packets merge/split and deliveries consume counts. Individual identity, shifts, boarding permissions and conserved manifests must be designed separately and integrated at movement boundaries.

## Scope and effort

One engineer-week (EW) is five focused working days by an experienced engineer, including implementation, review and verification. Four EW are called an engineer-month below for rough comparison; calendar months are different. Estimates assume senior Rust/C++ capability, an explicit compatibility matrix, access to upstream fixtures and no full redesign of the game economy.

Ranges include ordinary integration work and uncertainty within the stated scope. They are not statistical confidence intervals. Confidence is low-to-moderate before the first executable parity slice. AI-assisted code drafting is assumed available, but no unmeasured speed multiplier is applied to semantic verification.

| Outcome | Total effort, cumulative | Illustrative staffed calendar | Included and excluded |
|---|---:|---|---|
| **A. Headless offshore subset with people** | **36–60 EW / 9–15 engineer-months** | **About 6–10 months with 2 engineers** | Fixed scenario/map, ports/docks, one ship family, orders, capacity, basic freight, person journeys, Grasida adapter, own snapshots and API. No complete terrain generator, rail/road/aircraft, full economy, NewGRF/Squirrel, legacy `.sav`, stock-client networking or polished UI. |
| **B. General headless base-game transport core** | **100–180 EW / 25–45 engineer-months** | **About 9–16 months with 4 engineers** | A plus all four transport modes, construction, supported base climates/economies, towns/industries, general freight/CargoDist, native snapshots and scale verification. Still no blanket NewGRF, old-save, script ecosystem, stock-client or complete GUI compatibility. |
| **C. Broad OpenTTD-compatible replacement** | **280–520 EW / 70–130 engineer-months** | **About 18–36 months with 6 engineers** | B plus explicitly bounded legacy-save/content/script/network compatibility, graphical application and cross-feature verification against the pinned reference. “Every historical mod/save/platform works” is not promised. |

Calendar ranges are illustrative, not effort divided by nominal headcount: assume roughly 65–75% effective parallel utilization, sequential foundation/acceptance gates and integration delays. A solo effort is approximately 9–15, 25–45 or 70–130 focused engineer-months respectively, before ordinary calendar interruptions. Changing scope, unfamiliar staff or lack of authoritative fixtures can exceed the ranges.

Full compatibility here means a named, published test matrix against a pinned upstream version. Maintaining parity with upstream after delivery is additional recurring work and is not included. An unchanged original OpenTTD multiplayer client cannot be treated as a generic viewer for a different Rust simulation: it executes its own simulation and must agree on commands/state behavior.

### A. Initial subset work breakdown

| Stage | EW | Main deliverable and exit evidence |
|---|---:|---|
| A0 — Ontology, compatibility contract and C++ oracle | 4–6 | Field/relationship inventory for the slice; IDs/units/state ownership; built upstream runner; normalized state export; first deterministic fixtures |
| A1 — Foundation | 6–10 | Typed IDs/arenas, map accessors, clocks, PRNG, command context, minimal entity lifecycle; primitive parity fixtures |
| A2 — Stations, orders and cargo | 8–14 | Docks, shared schedules, cargo ownership/split/merge, loading/unloading and capacity; conservation/roundtrip fixtures |
| A3 — Ship movement slice | 6–10 | Fixed water network, ship movement/pathfinding and dock interactions; C++/Rust trace comparisons within declared scope |
| A4 — People and host integration | 6–10 | Persistent identity, journeys, reservation/manifest policy, asynchronous authorization inputs, denial consequences; person invariants |
| A5 — Core API, snapshots and hardening | 6–10 | Restart/replay, versioned API, cached observations, reconnect/idempotency, scale profiles and failure scenarios |
| **Total A** | **36–60** | Headless operational slice, not a full game |

This assessment is a starting input to A0; it does not mean the 4–6 EW discovery/oracle stage is complete. Optional helicopter operations within the otherwise restricted offshore slice would add roughly 4–8 EW after the ship slice; this is an engineering allowance, not measured throughput, and is already subsumed by the broader aircraft budget in B.

### B. Additional work after A

| Workstream | Additional EW |
|---|---:|
| Road vehicles, road/tram infrastructure and junction behavior | 8–14 |
| Rail construction, consists, signals, reservations and routing | 20–36 |
| Aircraft, airport movement/state machines and related rules | 8–14 |
| Towns, industries, generation and base climate behavior | 10–20 |
| General economy, ratings, subsidies and cargo distribution | 8–16 |
| Cross-mode integration, scale, restart and regression hardening | 10–20 |
| **Additional B** | **64–120** |
| **A + B cumulative** | **100–180** |

### C. Additional work after B

| Compatibility workstream | Additional EW |
|---|---:|
| NewGRF semantics, callback execution, persistent state and content tests | 40–80 |
| Legacy `.sav` codecs, migrations and supported-version corpus | 16–32 |
| AI/GameScript API and Squirrel runtime integration/compatibility | 20–36 |
| Multiplayer protocol, determinism, joins and desync handling | 20–36 |
| GUI, rendering, audio, localization and selected platform packaging | 30–60 |
| Compatibility combinations, performance and release qualification | 54–96 |
| **Additional C** | **180–340** |
| **A + B + C cumulative** | **280–520** |

These workstreams overlap in code but have distinct deliverables: for example NewGRF parsing/callbacks belong to content compatibility, whereas combinations of NewGRF + aircraft + load/rejoin belong to qualification. If retaining a C++ Squirrel runtime or other dependency is acceptable, “Rust core with native dependencies” is the result; an all-Rust replacement of every dependency is a separate scope, not included here.

## Translation sequence

Start from the ontology in [domain-model.md](002-ontology-and-data-model.md), and preserve behavior in small slices:

1. **Name and classify:** entity, definition, ID, unit, relationship, cache, presentation and external input. Trace creation, mutation, deletion and save/load for each slice entity.
2. **Freeze the behavioral reference:** exact commit, settings, content hashes, RNG states, command log, clocks and canonical state projections.
3. **Port primitives and ownership:** retain integer semantics and packed map access initially; introduce explicit world ownership and typed references.
4. **Port one end-to-end transport slice:** command → station queue → load → ship movement → unload/delivery. Compare traces at fixed tick boundaries.
5. **Add people in a separate ruleset mode:** legacy freight remains comparable to upstream; person identity/authorization has its own invariants and deliberately different behavior.
6. **Broaden modes and compatibility only after gates pass.** A vehicle moving on screen is not evidence that shared orders, save/load, capacity, transfer or deletion semantics are correct.

Avoid a monolithic “translate all headers, then all functions” phase. It produces a large amount of code before the first observable behavioral test. Also avoid a simultaneous ECS redesign, floating-point movement rewrite and CargoDist replacement: each would make differences harder to classify.

## First four-week decision gate

Suggested first investment: two engineers for four calendar weeks, with work organized around A0 and the first foundation fixtures. This is a calibration gate, not a promise to finish A1 or the ship slice.

| Week | Reviewable output |
|---|---|
| 1 | Reproducible dedicated C++ build; fixed map/content manifest; ontology for ship, dock, orders, cargo and clocks; concrete exclusions |
| 2 | C++ canonical trace exporter; native model sketches; PRNG/date/tile/ID fixture runner |
| 3 | Minimal Rust world and command harness; representative primitive and cargo comparisons; first loading/movement boundary experiment |
| 4 | First-divergence report, restore experiment, measured implementation/verification effort and revised budget |

Before starting, set a modest scenario target, for example 256² map, 10 ships, 100 named people and a seven-operational-day scenario. These are proposed acceptance inputs, not measured capacity claims. The gate should include at least one denied boarding, one full vessel, one transfer, one save/reload, one deleted destination and one interrupted external request.

Proceed when the fixture runner can identify the first divergent tick/field, primitive semantics agree, and the team can explain the first transport boundary differences. If the exporter or dependencies prevent that, revise the boundary before translating more code. No arbitrary “80% ported” progress metric: report fixture/scenario coverage and supported behaviors.

## Main risks and how the estimate changes

| Risk | Consequence | Required decision/evidence |
|---|---|---|
| “Faithful” remains undefined | Unbounded content/save/gameplay work | Explicit compatibility matrix and pinned upstream |
| Clock/PRNG/iteration differences | Desync and drifting outcomes even when code looks equivalent | Exact primitive tests and first-divergence traces |
| Core/UI/global coupling | Hidden state mutations survive attempted extraction | World-state ownership audit and notification classification |
| Passenger counts mistaken for people | Duplicate/lost identity or invented arrival history | Separate person store and single-location invariant |
| Stock OpenTTD client required for Rust server | Network protocol alone is insufficient | Treat cross-implementation simulation parity as C scope |
| Port plus redesign plus new features | Hard to attribute differences | Separate compatibility mode and people mode |
| Historical saves/mods treated as parse-only | Late failures in migrations and callback execution | Corpus-driven supported-version/content matrix |
| Assume Rust automatically speeds simulation | Poor architectural choices and optimistic deadlines | Benchmark named scenarios; preserve useful compact layouts |
| Live Grasida time differs from replay time | Invalid/backdated external actions | Explicit host clock mapping and recorded authorization inputs |

A lower-cost comparator is to keep upstream as a process and patch only named journeys/boarding/events. A preliminary allowance is **8–16 EW** for a restricted ship/dock demonstrator with a Rust adapter, including persistence and essential failure checks; it is not equivalent to A's reusable Rust core, nor a full generic person system. This comparator should be calibrated with the same first scenario before committing to a complete translation.

## What remains unverified

- No dedicated build, cargo fixture runner, executable Rust models or performance benchmark was run.
- No transitive extraction boundary has been proven by a linker/build experiment.
- Full field-level ontology, callback reachability, serialization mappings and deletion paths remain A0 work.
- Effort numbers are judgment-based work-package ranges. They are not inferred from the knowledge graph, commit count or line count.
- The upstream source and its attribution/license remain the reference for any future translation; dependency/runtime choices need to be recorded with the implementation scope.

The current deliverable is a reproducible inventory, a source-grounded initial domain design, and an estimate that can be revised using a small executable slice.
