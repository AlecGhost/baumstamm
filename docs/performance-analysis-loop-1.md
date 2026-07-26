# Performance analysis after optimization loop 1

Date: 2026-07-26  
Baseline report: `docs/performance-analysis-baseline.md`  
Equivalence gate: `a4cdfcb` (`test(lib): snapshot validation behavior`)  
Optimization under test: `783c19c` (`perf(lib): index consistency validation`)  
Status: loop 1 complete; proceed with a narrow view-indexing loop 2

## Executive summary

Loop 1 removed consistency validation as Baumstamm's dominant scaling problem.
On the same Darwin arm64 host and the same Criterion fixtures, parse plus
validation improved as follows:

| Fixture                        | Baseline | After loop 1 | Speedup | Reduction |
| ------------------------------ | -------: | -----------: | ------: | --------: |
| 46 people / 31 relationships   | 292.97 µs |    29.124 µs |   10.1x |     90.1% |
| 190 people / 127 relationships | 4.8096 ms |    112.00 µs |   42.9x |     97.7% |
| 766 people / 511 relationships | 87.869 ms |    469.81 µs |     187x |    99.47% |

Scaling is now close to the expected linear regime. Approximately four times
as many relationships takes 3.85 times and then 4.19 times as long, compared
with the baseline's 16.4 times and 18.3 times. The comparable level-10 CLI
workflow fell from about 3.5 seconds to 0.06–0.07 seconds across five runs,
roughly a 50–58x end-to-end improvement. Its 436,983-byte output is
byte-identical to the baseline, with SHA-256:

```text
c5f37472a9e67421c2561bb43c1b064a52af8bfa35b4ce5460d7732d6cf6fbca
```

The short post-change samply capture contains only 91 samples, which is enough
to demonstrate that the former multi-second validation workload disappeared,
but not enough to rank the remaining functions. A larger level-12 diagnostic
run contains 1,028 samples and repeatedly resolves Baumstamm frames to
`View::collect_descendents`, membership-vector construction in `View::new`,
and `filter_persons`. Allocator activity remains visible. Together with the
baseline's superlinear view benchmarks, that is sufficient evidence to proceed
to loop 2, limited to operation-local view indexes and membership sets.

The view implementation was not changed in loop 1. Its post-change short-run
estimates were 37.640 µs, 332.73 µs, and 4.6886 ms. Criterion reported apparent
changes of +9.35%, +7.71%, and +11.70% against the immediately retained
comparison base. These figures should be treated as measurement variance and a
regression watch, not as evidence that validation indexing caused a view
regression. The published baseline point estimates and Criterion's retained
statistical base came from different short runs, and no code on the measured
`View::new` path changed.

## What changed

`783c19c` replaces repeated whole-vector scans and temporary descendant vectors
with one `RelationshipIndex` local to each consistency check:

- relationship and parent-pair uniqueness use pre-sized `HashSet`s;
- referenced-person and unique-child state is collected while scanning the
  relationships once;
- `relationships_by_person` represents each relationship as a hyperedge and
  lets connectivity use breadth-first search without rescanning every
  relationship for every person;
- `children_by_parent` represents directed parent-to-child edges and lets
  Kahn's topological algorithm detect indirect cycles once for the whole
  graph;
- the top-level person-list checks reuse the referenced-person set instead of
  extracting and deduplicating the same IDs again; and
- the old allocation-heavy `Relationship::descendants` helper is no longer
  needed in production.

The index is deliberately operation-local. Canonical storage remains ordered
`Vec`s, while hash-backed structures are lookup-only, so nondeterministic hash
iteration does not become serialized output order. There is no cache lifecycle
or invalidation protocol.

The broad average-time complexity of validation changes from repeated,
superlinear graph scans to `O(P + R + E)`, where `P` is the number of referenced
people, `R` the number of relationships, and `E` the number of person
references/parent-child edges. Each relationship hyperedge is expanded at most
once during connectivity, and Kahn's algorithm visits each directed edge once.
Hash operations are average `O(1)`.

The tradeoff is transient `O(P + R + E)` auxiliary memory for sets, maps,
adjacency vectors, the connectivity queue, relationship-visited bitmap, and
indegrees. UUID hashing and fragmented map storage have worse constants and
locality than a dense integer index. The measurements show that this cost is
overwhelmingly favorable to repeated scans and short-lived vectors at current
sizes. A dense index would be a larger change and is not warranted by the new
profile.

## Benchmark results

### Parse and consistency validation

The designated post-loop point estimates and direct comparison with the
baseline report are:

| Fixture | Baseline point estimate | Loop-1 point estimate | Baseline / loop 1 | Time removed |
| --- | ---: | ---: | ---: | ---: |
| 46 / 31 | 292.97 µs | 29.124 µs | 10.06x | 90.06% |
| 190 / 127 | 4.8096 ms | 112.00 µs | 42.94x | 97.67% |
| 766 / 511 | 87.869 ms | 469.81 µs | 187.03x | 99.465% |

The increasing speedup is expected: the baseline paid for repeated scans and
overlapping descendant traversals, whereas the replacement constructs and
traverses the graph once. The largest fixture is now about 4.2 times slower
than the middle fixture for about four times as many relationships.

### Preparsed view construction

The corresponding post-loop short-run `View::new` estimates are:

| Fixture                        | After loop 1 | Criterion-reported change |
| ------------------------------ | -----------: | ------------------------: |
| 46 people / 31 relationships   |    37.640 µs |                    +9.35% |
| 190 people / 127 relationships |    332.73 µs |                    +7.71% |
| 766 people / 511 relationships |   4.6886 ms |                   +11.70% |

These comparisons do not establish a causal regression. `783c19c` changes
consistency checking, while this benchmark constructs a view from a preparsed
tree and excludes validation. The sample size is 10 with only one second of
measurement time, and the small-fixture timings have shown run-to-run
variation. Loop 2 should retain these numbers as a watch and use repeated
Criterion runs; its acceptance criterion must be a statistically and
practically meaningful improvement on the larger fixtures.

## End-to-end and profile results

### Comparable level-10 workload

The level-10 workload has 3,070 people, 2,047 relationships, and an input size
of 220,016 bytes. Five release-mode CLI runs took 0.06–0.07 seconds each,
compared with about 3.5 seconds at baseline. The generated view is 436,983
bytes and matches the baseline byte-for-byte at the SHA-256 shown above.

The post-loop profile contains 91 samples, down from 3,514 in the baseline
capture. Samply samples at a fixed interval, so this is primarily another
measure of the much shorter run, not a percentage breakdown of functions.
The former repeated `Relationship::descendants` validation stacks are no
longer the useful target.

### Larger level-12 diagnostic workload

Because level 10 completes too quickly for useful statistical attribution, a
level-12 synthetic workload was also profiled:

| Property | Value |
| --- | ---: |
| People | 12,286 |
| Relationships | 8,191 |
| Input | 908,138 bytes |
| Output | 1,776,369 bytes |
| Samply samples | 1,028 |
| Three release CLI runs | 0.98–1.03 seconds |

The counts are derived directly from the generated JSON and agree with the
fixture generator: a complete level-12 binary hierarchy has 8,191
relationships and 12,286 people, including partners.

With local debug symbols resolved, Baumstamm frames repeatedly lead through
`View::collect_descendents`, the parent/child membership-vector work in
`View::new`, and `filter_persons`. Allocator frames are still prominent.
This agrees with code inspection: descendant expansion scans all relationships
for every visited person and calls the allocating `parents()` helper;
`View::new` builds vectors and uses linear `contains`; and `filter_persons`
builds a person-ID vector before scanning all persons with another linear
membership test.

The level-12 capture is diagnostic rather than a baseline/post comparison:
there is no retained level-12 baseline profile. It identifies the next broad
target but does not assign reliable exclusive percentages to individual view
functions.

## Equivalence and verification

The snapshot gate was committed before the optimization in `a4cdfcb`. It
covers accepted-tree round trips, balanced valid trees at several depths, all
consistency error variants used by the suite, and precedence between multiple
simultaneous faults. After `783c19c`:

- `cargo insta test -p baumstamm-lib --check` passes with no snapshots to
  review;
- `cargo test --workspace` passes: all workspace unit, integration, and doc
  tests are green;
- the level-10 CLI output is byte-identical to the retained baseline; and
- `git diff a4cdfcb..783c19c --check` reports no whitespace errors.

The implementation preserves the existing validation order: duplicate
relationship ID, duplicate two-parent relationship, self-reference, direct
cycle, missing child, duplicate child, disconnected tree, indirect cycle,
duplicate person ID, then person-list quantity/membership. Kahn's algorithm
answers the same validity question as checking whether a relationship's parent
is among its transitive descendants; snapshots lock the observable error and
precedence behavior.

Strict workspace clippy is not green:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

It stops at the pre-existing `manual_is_multiple_of` warning in
`baumstamm-lib/src/graph.rs:581` (`len % 2 == 0`). This line is outside the loop
1 diff. The report-only re-analysis does not mix that unrelated cleanup into
the independently removable optimization commit.

## UI implications

The optimization substantially reduces native Rust work that also runs inside
the application's WASM worker. Parsing, mutation validation, and conversions
that validate a `FamilyTree` should therefore occupy the worker for much less
time on large trees. The level-10 CLI result also shows that the complete
parse/view/second-validation/serialization path can benefit dramatically.

This still does not prove that the browser UI cannot pause. Worker-to-JavaScript
conversion, structured cloning, React reconciliation, DOM construction,
layout, and paint are absent from these native measurements. Loop 1 made no
WASM protocol or threading change. Browser traces remain necessary before
attributing user-visible latency to the boundary or renderer.

## Proposal for optimization loop 2

Proceed with one narrow, independently committed view optimization:

1. Before changing `View`, add and commit snapshot coverage for representative
   roots, overlapping ancestry/descendancy, finite and unlimited generation
   limits, every meaningful sibling/partner option, missing-parent
   relationships, and observable relationship/child/person ordering.
2. Inside one `View::new`, build operation-local indexes from the existing
   ordered relationships: child-to-origin relationship and
   parent-to-partnership relationships. Preserve source order in each
   adjacency list.
3. Pass those indexes through ancestor and descendant collection so traversal
   no longer scans the complete relationship vector for every person. Avoid
   allocating `parents()` vectors in the traversal.
4. Replace the `parent_pids`, `child_pids`, `missing_parent`, and
   `filter_persons` linear membership vectors with sets used only for lookup.
   Continue producing persons and relationships in their current observable
   order.
5. Re-run unchanged snapshots, workspace tests, Criterion, the comparable
   level-10 CLI/hash check, and the level-12 profile. Continue to another loop
   only if that profile demonstrates a new dominant 80-percent target.

Expected average complexity for the lookup portion of a full tree-like view
falls from repeated relationship scans and linear memberships toward
`O(P + R + E)` with transient `O(P + R)` index/set storage. Relationships that
enter the output still need owned clones because view options can redact
parents and siblings; the goal is to discover and clone each selected
relationship once, not to redesign ownership.

Loop 2 should not add Rayon, a persistent `FamilyTree` cache, incremental
invalidation, a dense-ID representation, a trusted validation bypass, or a
WASM/TypeScript protocol change. None is needed to address the newly measured
view lookup hotspot, and each would make semantic equivalence or independent
removal harder.

## Reproduction

Run the isolated benchmarks from the repository root:

```sh
cargo bench -p baumstamm-lib --bench performance
```

Verify snapshots, tests, and lint:

```sh
cargo insta test -p baumstamm-lib --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Generate the comparable level-10 profile:

```sh
scripts/profile-cpu-samply.sh \
  --output-dir /tmp/baumstamm-post-validation-level10
```

Generate the larger diagnostic profile:

```sh
BAUMSTAMM_PROFILE_LEVELS=12 \
  scripts/profile-cpu-samply.sh \
  --output-dir /tmp/baumstamm-post-validation-level12
```

Time an already-built release binary without the profiler:

```sh
/usr/bin/time -p target/release/baumstamm-cli \
  /tmp/baumstamm-post-validation-level10/workload.json \
  view 0 \
  --show-partners \
  --show-siblings \
  --show-ancestor-siblings \
  --show-partner-siblings \
  > /tmp/baumstamm-post-validation-level10-timed.json
```

Check artifact sizes, counts, output identity, and sample counts:

```sh
wc -c /tmp/baumstamm-post-validation-level{10,12}/{workload,view-output}.json
jq '{relationships: (.relationships | length), persons: (.persons | length)}' \
  /tmp/baumstamm-post-validation-level{10,12}/workload.json
shasum -a 256 /tmp/baumstamm-post-validation-level10/view-output.json
jq '.threads[0].samples.length' \
  /tmp/baumstamm-post-validation-level{10,12}/profile.json
```

Inspect locally resolved call trees and flame graphs with:

```sh
samply load /tmp/baumstamm-post-validation-level12/profile.json
```

## Limitations

- Criterion uses synthetic balanced trees, sample size 10, a 500 ms warm-up,
  and a one-second measurement interval. Small differences require repeated
  runs, and real genealogies can have different depth, width, and overlap.
- The direct speedups above compare the designated report point estimates.
  Criterion's reported change percentages use its immediately retained base,
  which may be a different short baseline run.
- The CLI wall times are rounded by `/usr/bin/time`, use warm filesystem/build
  state, and are not a calibrated latency distribution.
- A 91-sample profile cannot rank remaining leaf functions. The 1,028-sample
  level-12 profile improves attribution but remains statistical.
- The saved profile declares itself unsymbolicated; the function attribution
  above depends on local resolution against the debug-symbol release binary.
  Moving the profile without that binary may show only addresses.
- Level 12 is a deliberately larger diagnostic workload and has no matching
  retained pre-loop profile, so only its post-loop attribution is claimed.
- Native arm64 results do not establish WASM constants, worker transfer cost,
  browser main-thread responsiveness, or React rendering performance.
- Hash-backed graph operations are average-time claims and consume more
  transient memory than the canonical vectors. Adversarial hashing and exact
  peak memory were not measured.

Loop 1 achieved the intended 80-percent result with wide margin. The evidence
supports one more focused loop on view lookup, protected by view snapshots and
kept separate from parallelism, caching, and protocol redesign.
