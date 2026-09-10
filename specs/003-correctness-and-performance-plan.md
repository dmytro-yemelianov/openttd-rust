# Correctness and performance repair plan

Status: Planned, 2026-09-10. No implementation is claimed by this document.

This plan addresses the Rust scaffold review before expanding the transport slice.
Upstream reference: OpenTTD `18fc940ebf4a837d8f510050774ec20a2acab63a`.
The workspace currently passes two map tests; simulation and persistence lack tests.
Review probes reproduced invalid order progression, acceptance of malformed map
storage followed by a panic, and tile sizes of 16 bytes (base) / 20 bytes (complete).

## Required safeguards when translating upstream behavior

User requirement: do not carry the identified upstream limitations into Rust by
mechanical translation. Apply these checks when each subsystem enters scope:

- **Routing complexity:** choose and document an algorithm with explicit work and
  memory bounds for the supported workload. Test growing and fragmented networks.
  Do not adopt the expensive CargoDist pass without evaluating alternatives and
  recording the behavior/performance tradeoff.
- **Routing scheduling:** keep routing work off the synchronous tick path or divide
  it into deterministic work units. Specify result publication ticks, graph revision
  checks, cancellation and a deterministic policy for unfinished jobs. Test replay
  with different worker completion timings. Wall-clock readiness must not silently
  change simulation outcomes; background execution alone does not prevent stalls.
- **Graph lifecycle:** test permanent splits, temporary disconnections, merges and
  repeated edits. Measure retained graph size and recomputation cost before choosing
  incremental component maintenance or periodic rebuilding.
- **Cache correctness:** treat caches as derived state. Include relevant rules and
  topology revisions in cache validity; enforce query limits on cached results too.
  Require cached/uncached equivalence tests, including maximum-cost boundaries and
  topology changes, before enabling a pathfinding cache.
- **Determinism:** define entity/timer ordering, integer semantics and PRNG ownership.
  Never use HashMap iteration order or worker scheduling to resolve gameplay ties.
  Compare repeated runs and save/reload runs at each tick.
- **State ownership:** keep mutable simulation state world-owned and independent of
  UI globals. Validate external state, rebuild caches on restore and measure actual
  layouts rather than copying upstream byte-size comments.

For every intentional departure from upstream, record its scope, reason and
regression fixture in the compatibility contract. Exact comparison applies to the
remaining compatible behavior; it must not force reproduction of a known defect.
These are implementation gates, not claims that the current scaffold satisfies them.

## Repair sequence

1. **Commands and observable failures.** Unify immediate and queued execution in
   one handler. Give submissions stable sequence IDs and return typed outcomes,
   including created entity IDs. Preserve FIFO for equal priorities; record the
   effective execution order. Unsupported commands must report failure without
   mutation. Test both entry paths, ordering, and failure reporting. Introduce
   deterministic queue limits only with explicit admission/defer semantics.

2. **Orders and deterministic ticks.** Add station destinations to orders and
   explicit execution state for waiting, travel and completion. Remove the ID
   threshold; validate referenced lists and destinations. Keep empty lists idle;
   constrain list length to representable valid indexes; advance only on completion.
   Specify wrap/end behavior and shared-list edit/delete semantics. Test empty and
   single-order schedules, WaitTime(100), IDs above 999, last-order transitions,
   invalid references and shared orders. Establish stable entity processing order
   before cargo contention or random-number use. Pin tick/overflow semantics.

3. **Validated state and arithmetic.** Validate map dimensions, allocation bounds,
   tile count, IDs, key/entity-ID agreement and references when loading. Reject
   malformed maps before exposing accessors; make adjacency behavior explicit for
   invalid coordinates. Ensure ID allocation never returns INVALID or overwrites
   an entity on exhaustion. Define checked cargo/money operations and tick overflow.
   Tests cover corrupt snapshots, boundary coordinates, exhausted IDs and underflow.

4. **Recoverable snapshots.** Introduce a versioned envelope containing world,
   people, pending commands, sequence counters, ruleset and future PRNG state.
   Write a complete new generation, flush it, then atomically publish it with the
   platform-appropriate durability steps. Retain the previous valid generation.
   Validate before replacing live state; rebuild derived indexes after load.
   Define migration or explicit rejection of old station `rating` snapshots.
   Test interrupted writes, unsupported versions, pending-command preservation and
   equality of continuous versus save/reload execution traces.

5. **One working transport slice.** Implement dock creation, purchase, schedules,
   movement, arrival and capacity-limited cargo transfer. Maintain a derived
   tile-to-station index on creation/deletion/load; resolve overlap policy explicitly.
   Replace no-op success paths with implemented behavior or explicit unsupported
   status. Test cargo conservation, insufficient stock/capacity, deleted destinations
   and dock-to-dock travel. Integrate people separately with boarding/location
   invariants; stock OpenTTD cannot serve as an oracle for named-person behavior.

6. **Measure and optimize.** Benchmark named map/fleet/station/queue sizes in release
   mode, recording hardware, commit, tick latency, allocations and memory. Remove
   per-tick key allocation/relookup while preserving the chosen deterministic order.
   Measure station indexing and queue alternatives before changing representations.
   Correct tile-size documentation now; select compact storage from measured memory
   needs and parity requirements. Verify identical traces after each optimization.

7. **Enforce gates.** Add CI for workspace tests and changed-code formatting/lints.
   Replace stale success claims with supported-behavior coverage. Remove unused
   imports and empty exports; do not delete planned public types merely because
   the scaffold does not call them yet. Require exact first-divergence comparisons
   for fields declared compatible, with intentional differences listed separately.

Steps 1–4 establish the safe foundation; step 5 depends on it. Capture baseline
measurements before step 6. Add regression tests with each repair, not at the end.

## What the original OpenTTD source actually says

These are observations about the pinned checkout, not newly reproduced upstream
bugs or a claim about the latest issue tracker.

- **CargoDist solver cost:** `src/linkgraph/mcf.h:42` describes first-pass complexity
  as exponential in node count and documents accuracy, saturation and recalculation
  interval tradeoffs. Profile large connected cargo networks if CargoDist enters
  scope; do not assume a Rust translation changes the algorithmic cost.
- **Routing can stall simulation:** `src/linkgraph/linkgraphschedule.cpp:169`
  pauses when a routing job will miss its scheduled join, then resumes when ready.
  This is deliberate synchronization behavior. Record job duration and wait time.
- **Graph splitting is intentionally absent:** `src/linkgraph/linkgraph.h:208`
  explains that connectedness checks and transient splits may cost more than
  retaining larger graphs. Revisit only with workloads that demonstrate a benefit.
- **Rail cache limitation is documented:** `src/pathfinder/yapf/yapf_costrail.hpp:44`
  notes maximum cost does not work with caching enabled and references FS#2900.
  This needs a reproducer/call-site audit before being called an active gameplay
  defect. Rail is outside the initial ship subset.
- **Determinism constrains refactoring:** `src/timer/timer_game_common.h:21`
  requires priority ordering and restricts random use in unprioritized callbacks.
  Preserve RNG consumption and update order; parallel execution is not automatically
  behavior-preserving.

The earlier source assessment in SPEC-001 also records distributed global state,
application/UI coupling, packed data semantics and save migration complexity.
Those are extraction and maintenance constraints, not evidence of dead code.
No upstream dead-code removal or performance rewrite is justified by this review.
