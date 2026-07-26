# Baseline performance analysis

Date: 2026-07-26  
Baseline code: `e4d689d` and its profiling/benchmark prerequisites  
Status: pre-optimization; no production optimization is included in this report

## Executive summary

Baumstamm's dominant measured cost is consistency validation, specifically the
indirect-cycle and connectivity checks. The level-10 whole-program workload
contains 3,070 people and 2,047 relationships in a 220,016-byte (about 215 KiB)
JSON file. An unprofiled release build takes about 3.5 seconds for the complete
CLI `view` command on the baseline Darwin arm64 machine. Samply captured 3,514
main-thread samples. The profile is dominated by allocation and deallocation,
with Baumstamm frames repeatedly leading through `Relationship::descendants`
during `FamilyTree` parsing/consistency checking.

The isolated benchmarks support that attribution:

| Fixture                        | Parse + validate | Preparsed `View::new` | Parse share of their sum |
| ------------------------------ | ---------------: | --------------------: | -----------------------: |
| 46 people / 31 relationships   |        292.97 µs |             34.208 µs |                    89.5% |
| 190 people / 127 relationships |        4.8096 ms |             310.69 µs |                    93.9% |
| 766 people / 511 relationships |        87.869 ms |             4.0696 ms |                    95.6% |

For roughly four times as many relationships at each step, parse-and-validate
becomes 16.4 times and then 18.3 times slower. `View::new` becomes 9.1 times and
then 13.1 times slower. Both paths are superlinear, but validation is already
about 22 times more expensive than view construction at the largest Criterion
fixture. The CLI also validates twice: once while parsing the input and once
when converting the selected `View` back into a `FamilyTree`. This makes
consistency checking the clear "80 percent" target.

The UI architecture already makes an important separation: WASM runs in a Web
Worker and requests are serialized, so long-running Rust/WASM functions do not
execute on the browser's UI thread. The computations are genuinely implemented
in Rust. This prevents the measured Rust validation work itself from blocking
input and paint. It does **not** prove that the UI can never freeze: converting
large Rust structures into JavaScript, structured-cloning worker replies, and
rendering a large React grid still happen across or on the main-thread boundary
and have not been measured by the native CLI profile.

The first optimization should replace repeated relationship scans and temporary
vectors in consistency checking with one per-check index and linear graph
algorithms. Parallelism, arena allocation, and WASM protocol redesign should
not precede that change. They add complexity without addressing the algorithmic
cause demonstrated by the baseline.

## Methodology and environment

### Environment

- Host: Apple Darwin 24.4.0, arm64 (`RELEASE_ARM64_T8103`)
- Rust: `rustc 1.97.0 (2d8144b78 2026-07-07)`
- Cargo: `cargo 1.97.0 (c980f4866 2026-06-30)`
- Profiler: samply 0.12.0
- Cargo release profile: optimization level 3
- Profiling build: release with `CARGO_PROFILE_RELEASE_DEBUG=2`
- Criterion configuration: sample size 10, 500 ms warm-up, 1 second
  measurement time

The benchmark point estimates above are the designated baseline run. Repeated
Criterion runs on this machine remain in the same performance regime but show
normal short-run variance, especially for the 46-person case. Optimization
comparisons must therefore use repeated runs, retain Criterion's statistical
comparison, and prioritize the larger fixtures.

### Reproduction

Install the pinned profiler once:

```sh
cargo install samply --version 0.12.0 --locked
```

Run the whole-program profile from the repository root:

```sh
scripts/profile-cpu-samply.sh \
  --output-dir /tmp/baumstamm-profile-baseline
```

The script deterministically generates the level-10 tree, builds the CLI with
release optimizations and debug symbols, and runs:

```sh
target/release/baumstamm-cli \
  /tmp/baumstamm-profile-baseline/workload.json \
  view 0 \
  --show-partners \
  --show-siblings \
  --show-ancestor-siblings \
  --show-partner-siblings
```

It writes the workload, captured view output, and `profile.json` without
opening or uploading the profile. Open the profile in Firefox Profiler to
inspect the call tree and flame graph.

Run the library benchmarks with:

```sh
cargo bench -p baumstamm-lib --bench performance
```

For a quick end-to-end wall-clock check after the script has generated a
fixture:

```sh
time target/release/baumstamm-cli \
  /tmp/baumstamm-profile-baseline/workload.json \
  view 0 \
  --show-partners \
  --show-siblings \
  --show-ancestor-siblings \
  --show-partner-siblings \
  > /tmp/baumstamm-view-output.json
```

The whole-program and Criterion workloads overlap intentionally but answer
different questions. The CLI profile attributes a real parse/view/serialize
workflow. Criterion separates `FamilyTree::try_from` from `View::new` on an
already parsed tree. Neither benchmark measures browser rendering or the WASM
boundary.

### Baseline artifact verification

The retained level-10 baseline artifact is 220,016 bytes and the generated CLI
output is 436,983 bytes. Its samply thread is named `baumstamm-cli` and contains
3,514 samples, matching the recorded baseline. The fixture generator's complete
binary hierarchy gives 3,070 people and 2,047 relationships. Samply is a
statistical profiler, so exact sample counts will vary on re-execution.

## Stage attribution

The CLI `view` action performs these stages:

1. Read the JSON file.
2. Deserialize it to `TreeData`.
3. Run `consistency::check` in `FamilyTree::try_from`.
4. Construct `View::new`.
5. Convert the view to owned `TreeData` and call `FamilyTree::try_from`, which
   runs the full consistency check again.
6. Serialize the result and print it.

The Criterion parse benchmark includes stages 2 and 3. Its preparsed view
benchmark includes only stage 4; it deliberately excludes the second validation
and output serialization. Consequently, adding the two Criterion numbers
understates the amount of validation in the whole CLI workflow.

The scaling is the strongest attribution evidence. Input size grows by about
four at each benchmark step. Linear JSON work should grow by about four.
Parse-and-validate instead grows by 16.4 and 18.3, close to quadratic scaling
with an additional allocation penalty. `View::new` is also superlinear, but its
absolute cost is much smaller. At 766 people, the measured parse/validation
path accounts for 95.6% of the sum of the two isolated timings. File I/O and
JSON serialization can matter for very large payloads, but neither explains
the profile's allocator/free dominance or the repeated descendant frames.

## Why consistency checking is expensive

### Indirect-cycle detection

`check_relationships` asks, for each relationship with a parent, whether any
parent occurs in `rel.descendants(relationships)`.

`Relationship::descendants` is a breadth-first traversal implemented without
an adjacency index:

- It clones the starting relationship's child vector.
- For every discovered descendant person, it scans **every relationship**.
- For every relationship in that inner scan, `rel.parents()` allocates a new
  `Vec<PersonId>` and `contains` scans it.
- Matching relationships clone their child vectors.
- `itertools::unique` creates deduplication state.
- Each candidate is checked against the growing descendant `Vec` with linear
  `contains`.
- The caller builds the complete descendant vector merely to ask whether it
  contains one of at most two parents.

This traversal is repeated independently for relationships throughout the
tree. On a balanced hierarchy, many descendant paths overlap, but none of the
work is shared. In broad terms, visiting descendants already accumulates
substantial repeated subtree work; scanning all relationships per visited
person adds another factor proportional to tree size. Linear membership checks
and allocation add further cost. Exact asymptotic behavior depends on tree
shape, but the benchmark's near-quadratic growth and allocator-heavy profile
are direct consequences of this structure.

Cycle detection does not need transitive descendant vectors. A directed graph
cycle algorithm (iterative depth-first colors or Kahn's topological algorithm)
can answer the same question in `O(V + E)` once parent-to-child adjacency is
built.

### Connectivity

`nr_connected_persons` has a similar pattern:

- For every person reached, it scans all relationships.
- `rel.persons()` allocates a vector for each tested relationship.
- A matched relationship calls `rel.persons()` again.
- `unique` allocates deduplication state.
- `total_related_persons.contains` is a linear visited check.

Connectivity can instead use the same indexed person adjacency with a
`HashSet`/visited bitmap and `VecDeque`, visiting each person and edge once.

### Other repeated work

The earlier checks are individually smaller but reinforce the allocation
pattern:

- `parents()` repeatedly turns a fixed two-element option array into a heap
  vector.
- Child IDs are cloned into vectors even though `PersonId` is `Copy`.
- `extract_persons` allocates and deduplicates a vector; it is called once in
  `check_relationships` and again in `check`.
- Relationship and person uniqueness are expressed through iterator adapters
  that create hash state, separately from later maps that need the same IDs.
- Canonical two-parent pairs are materialized as sorted vectors.
- The `persons_hashmap` stores `()` values where a set is the intended
  abstraction.

The top-level `Vec<Relationship>` and `Vec<Person>` are not themselves a
problem. They are compact and cache-friendly for serialization and ordered
iteration. The problem is repeatedly treating them as lookup structures.

## View construction

`View::new` is the secondary measured hotspot and also lacks indexes:

- Every ancestor step scans relationships to find the relationship containing
  the person as a child.
- Every descendant step scans relationships and calls the allocating
  `parents()` helper to find partnerships.
- Relationship objects and child vectors are cloned while collecting the
  result.
- Parent/child and missing-parent lists use vectors plus linear `contains`.
- `filter_persons` builds a person-ID vector, then scans all persons and uses
  linear membership checks.

`Traversal` and `RelationshipCollector` already use hash maps to avoid
unbounded repeated traversal and duplicate output relationships. That is why
the implementation is better than a fully naive recursive traversal, but the
lookup at each visited person is still a full relationship scan. The measured
9.1x and 13.1x growth for approximately 4x input confirms that an index would
also help here.

View output necessarily owns relationships because options can remove parents
or siblings and because it is later converted into an owned `FamilyTree`.
Avoiding all clones is therefore neither possible nor desirable. The useful
change is to clone each selected item once, not to clone temporary vectors
during discovery.

## Allocation, cache behavior, and data structures

The flame graph's allocator/free dominance is expected from temporary
`Vec<PersonId>` creation in the deepest loops. Each allocation adds allocator
metadata work and touches memory that is soon discarded. Repeated full scans of
the relationship vector are sequential within each scan, but revisiting it for
each person expands the working set and repeatedly reloads fields. Growing
descendant vectors plus linear membership checks introduce additional cache
traffic.

A per-operation index is the appropriate first data structure:

- `HashSet<RelationshipId>` and `HashSet<PersonId>` for uniqueness/membership;
- a child-to-origin map to enforce exactly one origin relationship and support
  ancestor lookup;
- parent-to-children or parent-to-partnership adjacency for cycle, descendant,
  and view traversal;
- undirected person adjacency, or adjacency derived from relationships, for
  connectivity;
- a canonical fixed pair for relationship-parent uniqueness.

Hash maps trade some locality and hashing cost for eliminating whole-vector
scans. With 128-bit UUID keys that is still overwhelmingly favorable at the
measured sizes. A denser alternative is to map IDs to contiguous integer
indices once, then keep adjacency in `Vec<Vec<usize>>` and visited state in
`Vec<bool>` or a generation-mark vector. That has better locality and lower
per-entry overhead, but is a larger change. Start with the simplest indexed
implementation that demonstrates the algorithmic win; consider dense indices
only if the follow-up profile shows hashing is material.

An arena is not the answer to the observed bottleneck. The objects already live
in stable top-level vectors and relationships refer to copyable IDs rather than
pointers. An arena could reduce allocation only if the representation were
redesigned and would introduce lifetime/invalidation complexity. A linked list
would be actively unfavorable: it adds an allocation per node, pointer chasing,
and cache misses while giving no useful insertion advantage to the current
append/retain workloads. Preserve contiguous vectors as canonical storage.

## Repeated operations and caching opportunities

There are three levels of possible reuse:

1. **Within one check or view construction:** build lookup tables once. This is
   low-risk, requires no invalidation protocol, and directly attacks the
   measured hotspot.
2. **Across internal conversion:** `View` comes from an already consistent
   `FamilyTree`, but `FamilyTree::from(view)` converts through `try_from` and
   validates again. A private trusted constructor could remove this duplicate
   check after equivalence is established. This must retain a clear invariant
   boundary; public/raw input must always validate.
3. **Across mutations and snapshots:** the WASM state recomputes partial views
   after mutations and recomputes the complete grid in `get_tree_data`.
   Persistent indexes or cached grids could help repeated reads, but every tree
   mutation and layout/view option change would need exact invalidation. This is
   not justified until re-profiling after the local algorithmic fixes.

Mutation methods currently run the full consistency check after small edits.
Making validation close to linear will improve them automatically. Incremental
validation may eventually be useful, but it greatly increases the number of
invariant-maintenance paths and should not be the first optimization.

## UI threading and the WASM boundary

The React application instantiates `baumstamm-wasm` inside
`wasm.worker.ts`. The worker:

- initializes the WASM module;
- invokes the exported Rust functions;
- serializes access through `SerialTaskQueue`;
- posts results back to the main thread.

The client also serializes requests. This is appropriate because the Rust WASM
module has a single global `Mutex<State>` and operations must observe a stable
order. It also provides backpressure rather than racing mutations and reads.
Long-running parsing, consistency, view, and grid computations therefore run
off the UI thread and are authored in Rust (`baumstamm-lib` and
`baumstamm-grid`), satisfying the central decoupling requirement for
computation.

There are qualifications:

- Standard WebAssembly execution in this setup is single-threaded within the
  one worker. The Rust `Mutex` protects state but does not create parallelism.
- `serde_wasm_bindgen::to_value` creates JavaScript objects in the worker.
- `postMessage` then structured-clones those object graphs to the UI thread;
  no transferable buffers are used.
- `get_tree_data` returns persons, relationships, and a potentially large,
  nested grid. `getTreeSnapshot` additionally returns the full person list,
  duplicating person data when the current view is full.
- React reconciliation, DOM creation, layout, paint, and the tree-navigation
  helpers execute on the UI thread.

Thus the UI should stay responsive while Rust computes, but a large reply or
large DOM update can still create a main-thread pause. The native CLI profile
contains none of these costs, so it cannot establish whether the WASM boundary
is a bottleneck. A browser Performance trace with worker tracks, long-task
markers, payload sizes, and separate timestamps around Rust computation,
`to_value`, `postMessage`, state update, and paint is required before changing
the protocol.

Potential later boundary improvements include compact typed arrays, returning
IDs plus normalized tables instead of repeated nested objects, transferring
buffers, avoiding the duplicate full-person payload, and patch/delta replies.
They are speculative until browser measurements show meaningful boundary or
rendering time.

## Sequential work, parallel work, and Rayon

Current validation and view traversal are sequential. Some checks appear
parallelizable in isolation—for example uniqueness sets, scanning relationships
for local errors, or computing descendants for different roots—but
parallelizing the current implementation would multiply concurrent allocation
and memory-bandwidth pressure while preserving the bad repeated-scan
complexity. It would also complicate deterministic error precedence.

After indexing, graph construction is a single pass and connectivity/cycle
traversals are naturally linear and mostly dependent. Their granularity at
hundreds or a few thousand nodes is likely too small for Rayon scheduling to
pay off. Native Rayon could plausibly help independent expensive layout
scoring or serialization-adjacent transforms on much larger trees, but that
must be benchmarked.

Rayon is especially low-value for the primary application target today:
multi-threaded WASM requires atomics, a worker pool, `SharedArrayBuffer`, and
cross-origin isolation headers. The existing application intentionally uses
one worker, and adding a pool would increase deployment and state-coordination
complexity. The correct order is:

1. remove the superlinear algorithm and allocations;
2. re-profile;
3. parallelize only a remaining coarse, independent CPU hotspot with benchmarks
   on both native and the actual deployed target.

No Rayon dependency should be added in the first optimization loop.

## Optimization proposal

### Equivalence gate: snapshots before changing optimized functions

Before modifying consistency or traversal logic, add a committed snapshot
corpus using the existing `cargo-insta` dependency. Snapshot:

- accepted `TreeData` normalized through `FamilyTree::save`;
- every existing consistency error fixture and exact error variant/message;
- balanced valid trees at multiple depths;
- view results for representative roots and all meaningful combinations of
  limits/sibling/partner options;
- overlapping ancestry/descendancy examples;
- relationship ordering and child ordering, because output ordering is
  observable through saved JSON and grids.

Existing exact-error unit tests and view snapshots are a useful start, not a
complete equivalence gate for a consistency rewrite. Review and commit all new
snapshots while the old implementation is still active. After each
optimization, the snapshots must be unchanged unless an intentional behavior
change is separately justified. Run:

```sh
cargo insta test -p baumstamm-lib
cargo test --workspace
```

### Loop 1: optimize consistency checking

Ordered implementation:

1. Build a local validation index in one pass over relationships and persons,
   using borrowed data and fixed parent arrays where possible.
2. Preserve the current error-check order exactly: relationship ID,
   duplicate parent pair, self-reference, direct cycle, missing child,
   duplicate child, connectivity, indirect cycle, then person-list checks.
3. Replace `nr_connected_persons` with indexed BFS/DFS and a hash/bitmap visited
   set.
4. Replace repeated `Relationship::descendants` calls with one directed cycle
   detection over parent-to-child edges.
5. Reuse the collected referenced-person set when comparing `TreeData.persons`.
6. Remove inner-loop `parents()`, `persons()`, child-vector clones, and linear
   visited membership without changing serialized order.

Expected impact: validation should move from near-quadratic toward `O(V + E)`
average-time graph work. Given its 95.6% share of the isolated large-fixture
sum and its duplicate use in the CLI, this should improve large parse/view CLI
workflows by at least several times and plausibly by an order of magnitude or
more. The actual target is determined by re-running Criterion and samply, not
by the estimate.

Acceptance:

- unchanged insta snapshots and error precedence;
- all workspace tests pass;
- Criterion reports a material parse-and-validate reduction at 190 and 766
  people with much flatter scaling;
- level-10 whole CLI output matches byte-for-byte or semantically according to
  the snapshot policy;
- new whole-program profile no longer centers on
  `Relationship::descendants`/allocator work.

### Loop 2: optimize view lookup if it becomes material

Only after Loop 1 re-profiling:

1. Build child-to-origin and parent-to-partnership lookup tables once per
   `View::new` (or expose an immutable index owned by `FamilyTree` if lifecycle
   evidence justifies it).
2. Use sets for missing-parent and selected-person membership.
3. Clone only relationships and children that enter the returned view.
4. Preserve traversal and result ordering to keep JSON/grid output stable.
5. Consider a private invariant-preserving conversion from `View` to
   `FamilyTree` to avoid redundant validation, but only if validation still has
   measurable impact after Loop 1.

Expected impact: `View::new` should approach linear traversal for tree-like
fixtures. Its 4.0696 ms baseline at 766 people is small compared with
validation, so optimizing it first would have poor end-to-end return.

### Loop 3: measure and optimize the next demonstrated 80%

Use the post-Loop-2 profile to choose exactly one dominant remaining area:

- grid graph/layout construction;
- JSON serialization;
- WASM-to-JS conversion and worker structured clone;
- React rendering/main-thread layout;
- or a still-visible library lookup.

For the browser path, add timing/trace instrumentation rather than inferring
from native measurements. If the boundary dominates, test compact transferable
representations against the current semantic snapshots. If grid generation
dominates, add a dedicated Rust benchmark and snapshot grid output before
changing it. Do not perform a third optimization merely because three loops are
allowed.

### Reporting after every loop

Each loop should be independently committed and followed by a new report or a
clearly versioned report section containing:

- exact commit and environment;
- Criterion point estimates and change percentages for every fixture;
- end-to-end level-10 wall time and samply sample attribution;
- output/snapshot equivalence results;
- regressions, tradeoffs, and the decision to continue or stop.

## Risks and limitations

- Samply has only 3,514 statistical samples in the recorded run. It is strong
  enough to identify the broad allocator/descendant hotspot, not to rank tiny
  leaf functions precisely.
- The profile is unsymbolicated at the artifact level on this host; attribution
  combines the profiler's visible call stacks with code inspection. Follow-up
  profiles should retain debug symbols and verify symbolication before making
  fine-grained claims.
- Criterion uses synthetic balanced trees. Real genealogies may be shallower,
  wider, contain multiple partnerships, or have overlapping paths. Add at least
  one representative real example benchmark before generalizing constants.
- Criterion's short measurement configuration is appropriate for iteration but
  produces visible run-to-run variance in small fixtures.
- Native arm64 timings do not predict WASM constants, browser serialization, or
  DOM performance.
- Hash maps have nondeterministic iteration order. Indexes must be used for
  lookup only, while observable output follows source/traversal order.
- A new cycle algorithm must preserve both validity semantics and current error
  precedence. Testing only valid trees would miss this risk.
- Persistent caches can become stale after mutation. Prefer operation-local
  indexes until their lifecycle and payoff are demonstrated.
- Removing the internal second validation tightens reliance on private
  invariants. It should be a separate, reviewable change rather than folded
  invisibly into the graph rewrite.
- Faster computation does not by itself guarantee a responsive UI after the
  worker replies. Browser long-task measurements are still required.

## Explicit non-goals for the first optimization

- No Rayon or WASM thread pool.
- No arena, linked-list, ECS, or pointer-based canonical tree rewrite.
- No change to public JSON, WASM, or TypeScript types.
- No persistent cache or incremental-validation subsystem.
- No change to validation rules, error order/messages, view ordering, or grid
  semantics.
- No speculative WASM protocol compression.
- No optimization of the new force-directed layout unless a post-validation
  profile identifies it as the dominant user-facing cost.
- No micro-optimization of serde, UUID formatting, file I/O, or CLI printing
  before the demonstrated consistency hotspot is removed.

The baseline supports a narrow first move: index once, validate with linear
graph traversals, preserve behavior with snapshots, and measure again.
