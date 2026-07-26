# Performance analysis after optimization loop 2

Date: 2026-07-26  
Loop-1 report: `cb4c313` (`docs(perf): report validation optimization results`)  
View equivalence gate: `0102498` (`test(lib): snapshot view behavior`)  
Optimization under test: `8a782d3` (`perf(lib): index view traversal`)  
Status: loop 2 complete; stop native optimization loops

## Executive summary

Loop 2 removed repeated whole-relationship scans and linear membership vectors
from view construction. On the same Darwin arm64 host and the same Criterion
fixtures, the designated `View::new` point estimates improved relative to the
loop-1 report as follows:

| Fixture | Loop 1 | After loop 2 | Speedup | Reduction |
| --- | ---: | ---: | ---: | ---: |
| 46 people / 31 relationships | 37.640 µs | 21.214 µs | 1.77x | 43.6% |
| 190 people / 127 relationships | 332.73 µs | 83.642 µs | 3.98x | 74.9% |
| 766 people / 511 relationships | 4.6886 ms | 336.23 µs | 13.94x | 92.8% |

The increasing speedup is the intended scaling result. Approximately four
times as many relationships now takes 3.94 times and then 4.02 times as long,
close to linear for these balanced fixtures. Against the original baseline
report, the same results are 1.61x, 3.71x, and 12.10x faster, reductions of
38.0%, 73.1%, and 91.7%.

Parse plus consistency validation was not intentionally changed in loop 2. Its
latest Criterion point estimates are 27.152 µs, 105.74 µs, and 450.25 µs.
These remain in the post-loop-1 performance regime; the largest run was noisy,
so small differences from the loop-1 report must not be attributed to the view
patch.

The comparable level-10 CLI workload still produces a 436,983-byte output
whose SHA-256 is byte-identical to both the baseline and loop 1:

```text
c5f37472a9e67421c2561bb43c1b064a52af8bfa35b4ce5460d7732d6cf6fbca
```

Five direct runs rounded to 0.00–0.01 seconds, compared with 0.06–0.07 seconds
after loop 1 and about 3.5 seconds at baseline. These coarse wall-clock figures
confirm that the workflow became much shorter, but their timer resolution does
not support a precise speedup ratio. Samply captured only 12 samples, versus 91
after loop 1 and 3,514 at baseline.

The larger level-12 diagnostic workload took 0.02–0.04 seconds across five
runs, down from 0.98–1.03 seconds after loop 1. Its 1,776,369-byte output is
byte-identical to loop 1, with SHA-256:

```text
29c6864154b523e83de897616d6d6be0ad0427b6e19f70c7c795e04e42ae87a9
```

Its profile contains only 53 samples, versus 1,028 after loop 1. That is too
little native execution for reliable samply attribution. Loop 3 is therefore
not justified: another speculative native change would violate the
80-percent principle. The next evidence-gathering step should be a browser
Performance trace that separates worker-side Rust/WASM time from
`serde_wasm_bindgen`, structured cloning, React reconciliation, DOM/layout,
and paint. This report does not implement that browser work.

## What changed

Commit `8a782d3` builds one operation-local `RelationshipIndex` for each
`View::new` traversal:

- `origin_by_child` maps each child to the source index of its origin
  relationship, replacing a complete relationship scan at every ancestor
  step;
- `partnerships_by_parent` maps each parent to source-ordered relationship
  indexes, replacing a complete relationship scan and allocating `parents()`
  call at every descendant step;
- ancestor and descendant traversal shares this index while retaining the
  existing generation-limit and overlap bookkeeping;
- parent, child, and missing-parent membership vectors became `HashSet`s;
- `filter_persons` uses a set of selected person IDs instead of a vector with a
  linear `contains`; and
- fixed-size parent arrays are traversed directly where temporary vectors are
  unnecessary.

The index and sets are deliberately operation-local. `FamilyTree` keeps its
ordered `Vec` storage, so there is no persistent cache or invalidation
protocol. Hash structures are used only for lookup. The adjacency vectors are
populated in source relationship order, selected relationships retain the
collector's established insertion and merge behavior, child order is retained,
and persons are still filtered in the tree's source order. Hash iteration order
therefore does not leak into serialized view output.

## Complexity and memory tradeoff

Let `P` be the number of persons, `R` the number of relationships, and `E` the
number of parent/child references. Before loop 2, each ancestor or descendant
step could scan all `R` relationships. Membership-vector construction and
person filtering added further linear searches, causing visibly superlinear
growth.

The new index is built in `O(R + E)` expected time. Ancestor lookup is expected
`O(1)` per visited person, and descendant traversal visits only indexed
partnerships and their child edges. Missing-parent detection, relationship
filtering, and person filtering use expected `O(1)` set membership. For the
balanced tree-like workloads measured here, broad view lookup and filtering
now approach `O(P + R + E)`, plus the unavoidable cost of cloning selected
owned relationships and children.

This is an average-time statement for standard hash maps and sets, not a
worst-case guarantee. The existing relationship collector can also revisit
children when overlapping traversal paths merge the same relationship, so
arbitrary highly overlapping graphs need not have a strict linear bound.

The tradeoff is transient `O(R + E + P)` auxiliary memory for:

- child-to-origin and parent-to-partnership indexes;
- parent, child, missing-parent, and selected-person sets;
- ancestor and descendant traversal state; and
- relationship collector lookup state.

That memory is released after the view is constructed. Hash tables have larger
constant factors and weaker locality than dense integer arrays, but the
measurements show that their cost is substantially lower than repeated scans
at the tested sizes. A persistent index or dense-ID redesign would add
lifecycle and representation complexity without evidence of a remaining
native bottleneck.

## Criterion results

### Preparsed view construction

The designated post-loop-2 Criterion slope point estimates and direct
comparison with the loop-1 report are:

| Fixture | Loop-1 point estimate | Loop-2 point estimate | Loop 1 / loop 2 | Time removed |
| --- | ---: | ---: | ---: | ---: |
| 46 / 31 | 37.640 µs | 21.214 µs | 1.77x | 43.64% |
| 190 / 127 | 332.73 µs | 83.642 µs | 3.98x | 74.86% |
| 766 / 511 | 4.6886 ms | 336.23 µs | 13.94x | 92.83% |

Compared with the designated original baseline:

| Fixture | Baseline point estimate | Loop-2 point estimate | Baseline / loop 2 | Time removed |
| --- | ---: | ---: | ---: | ---: |
| 46 / 31 | 34.208 µs | 21.214 µs | 1.61x | 37.99% |
| 190 / 127 | 310.69 µs | 83.642 µs | 3.71x | 73.08% |
| 766 / 511 | 4.0696 ms | 336.23 µs | 12.10x | 91.74% |

The smallest loop-1 estimate happened to be slower than the original baseline
estimate even though the measured view path had not changed in loop 1. The
post-loop-2 small-fixture speedup should therefore be read together with the
larger fixtures and scaling curve, not in isolation.

A repeated post-optimization Criterion run retained an immediately preceding
post-optimization base. Its median change estimates were approximately -0.50%,
+0.09%, and +0.43%; the middle and largest 95% confidence intervals crossed
zero. This is a stability check, not the loop-1 comparison, because Criterion's
retained base had already been replaced by loop-2 code.

### Parse and consistency validation

The latest parse-plus-validation slope point estimates were:

| Fixture | Loop 1 | Latest loop-2 run | Difference from loop 1 |
| --- | ---: | ---: | ---: |
| 46 / 31 | 29.124 µs | 27.152 µs | -6.8% |
| 190 / 127 | 112.00 µs | 105.74 µs | -5.6% |
| 766 / 511 | 469.81 µs | 450.25 µs | -4.2% |

Commit `8a782d3` changes `View` and is not on the parse/validation benchmark
path. These small apparent improvements are measurement drift, not a claimed
optimization effect. The largest fixture was particularly noisy: its retained
run had a wide distribution and its mean was pulled above its median and slope
estimate.

Relative to the original baseline, the latest values are approximately 10.79x,
45.49x, and 195.16x faster, reductions of 90.73%, 97.80%, and 99.49%. Those
large improvements remain attributable to loop 1's consistency index, not to
the loop-2 view patch.

## End-to-end and profile results

### Comparable level-10 workload

The level-10 workload remains 220,016 input bytes with 3,070 people and 2,047
relationships. The generated view is 436,983 bytes. Its hash matches the
retained baseline and loop-1 output exactly.

Five release-mode direct runs reported 0.00–0.01 seconds each. The same
workflow reported 0.06–0.07 seconds after loop 1 and about 3.5 seconds before
optimization. Because `/usr/bin/time` reports hundredths of a second here,
sub-tick and one-tick results cannot establish whether the real time is near
one millisecond or near ten milliseconds. It is valid to conclude that the
workflow is below the timer's useful resolution and materially shorter than
loop 1; it is not valid to publish an exact 6x, 60x, or larger ratio from these
rounded values.

Samply captured 12 samples, compared with 91 after loop 1 and 3,514 at
baseline. Samply uses fixed-interval statistical sampling, so the falling count
primarily reflects shorter execution. Twelve samples cannot rank residual
functions or assign percentages.

### Larger level-12 diagnostic workload

The level-12 fixture and results are:

| Property | Value |
| --- | ---: |
| People | 12,286 |
| Relationships | 8,191 |
| Input | 908,138 bytes |
| Output | 1,776,369 bytes |
| Output SHA-256 | `29c6864154b523e83de897616d6d6be0ad0427b6e19f70c7c795e04e42ae87a9` |
| Five release CLI runs | 0.02–0.04 seconds |
| Samply samples | 53 |

The input and output sizes agree with the retained loop-1 fixture and output,
and the output hash is unchanged. Loop 1 took 0.98–1.03 seconds and yielded
1,028 samples for this diagnostic workload. The new 53-sample capture verifies
that the previously visible view workload has collapsed, but is insufficient
to identify a new dominant native leaf function.

## Equivalence and verification

The view snapshot gate was committed before the optimization in `0102498`. It
covers:

- default unlimited views;
- finite ancestor and descendant generation limits, including zero;
- the complete Boolean option matrix for partners, siblings, ancestor
  siblings, and partner siblings;
- missing-parent synthesis;
- non-topological source ordering for relationships, children, and persons;
  and
- overlapping royal ancestry and descendancy with both finite and unlimited
  traversal.

After `8a782d3`:

- `cargo insta test -p baumstamm-lib --check` passes without snapshot changes
  or pending reviews;
- `cargo test --workspace` passes, including workspace unit, integration, and
  documentation tests;
- the level-10 output is byte-identical to the baseline and loop 1;
- the level-12 output is byte-identical to loop 1; and
- `git diff 0102498..8a782d3 --check` reports no whitespace errors.

Strict workspace clippy is still not green:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

It stops at the pre-existing `manual_is_multiple_of` warning in
`baumstamm-lib/src/graph.rs:581` (`len % 2 == 0`). That line is outside the
snapshot and optimization commits. Mixing an unrelated lint cleanup into the
independently removable performance patch would weaken its auditability.

The snapshots and output hashes protect observable behavior; they do not
formally prove equivalence for every valid graph. Code inspection additionally
confirms that index adjacency retains source order, child-origin lookup keeps
the first source match as the old `find` did, and sets are never iterated to
produce serialized order.

## UI implications

The native Rust work performed inside the WASM worker should benefit from both
optimization loops. On large views, loop 2 removes the same repeated scans and
temporary membership vectors that the level-12 native profile identified after
loop 1. The queued worker continues to keep Rust computation off the browser
main thread.

Native CLI results cannot establish browser responsiveness or total view
latency. They exclude:

- Rust-to-JavaScript conversion through `serde_wasm_bindgen`;
- worker message serialization and structured cloning;
- JavaScript object materialization and garbage collection;
- React reconciliation and component work; and
- DOM creation, style calculation, layout, paint, and compositing.

Now that the remaining native stage is below useful samply attribution at the
tested sizes, these boundary and renderer stages are more plausible places for
the next user-visible 80-percent cost. That is an inference and requires browser
measurement.

## Decision: stop before loop 3

Do not perform a third native optimization loop now. Loop 2 achieved the
planned algorithmic change and reduced the largest isolated view fixture by
92.8% relative to loop 1. The representative level-10 and larger level-12
profiles now have only 12 and 53 samples. They do not provide enough evidence
to rank another native target.

Potential changes such as dense IDs, a persistent `FamilyTree` index,
incremental caches, parallel traversal, alternate hashers, reduced cloning, or
collector redesign could all be made, but none currently has evidence that it
accounts for most remaining user-visible time. Implementing one speculatively
would add complexity and make semantic equivalence harder to establish while
violating the requested focus on the dominant 80 percent.

The next evidence step is a browser Performance trace on a representative
large tree. Instrument request boundaries so the trace can distinguish:

1. worker-side Rust/WASM parse, validation, and `View::new`;
2. `serde_wasm_bindgen` conversion and worker-side reply preparation;
3. `postMessage`/structured-clone transfer;
4. main-thread state update and React reconciliation; and
5. DOM/layout/paint.

Compare a level-10 or level-12 import/view operation with the UI idle baseline,
record payload sizes and long tasks, and optimize only the stage that dominates
the trace. This is a measurement proposal, not authorization or implementation
of a UI/protocol change.

## Reproduction

Run the isolated benchmarks from the repository root:

```sh
cargo bench -p baumstamm-lib --bench performance
```

Verify snapshots, tests, lint, and patch whitespace:

```sh
cargo insta test -p baumstamm-lib --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff 0102498..8a782d3 --check
```

Generate the comparable level-10 profile:

```sh
scripts/profile-cpu-samply.sh \
  --output-dir /tmp/baumstamm-post-view-level10
```

Generate the larger level-12 profile:

```sh
BAUMSTAMM_PROFILE_LEVELS=12 \
  scripts/profile-cpu-samply.sh \
  --output-dir /tmp/baumstamm-post-view-level12
```

Time an already-built release binary without the profiler:

```sh
/usr/bin/time -p target/release/baumstamm-cli \
  /tmp/baumstamm-post-view-level10/workload.json \
  view 0 \
  --show-partners \
  --show-siblings \
  --show-ancestor-siblings \
  --show-partner-siblings \
  > /tmp/baumstamm-post-view-level10-timed.json
```

Repeat with the level-12 workload by changing both paths from `level10` to
`level12`.

Check artifact sizes, fixture counts, hashes, and sample counts:

```sh
wc -c /tmp/baumstamm-post-view-level{10,12}/{workload,view-output}.json
jq '{relationships: (.relationships | length), persons: (.persons | length)}' \
  /tmp/baumstamm-post-view-level{10,12}/workload.json
shasum -a 256 \
  /tmp/baumstamm-post-view-level{10,12}/view-output.json
jq '.threads[0].samples.length' \
  /tmp/baumstamm-post-view-level{10,12}/profile.json
```

Inspect a locally resolved profile if a longer future workload produces enough
samples:

```sh
samply load /tmp/baumstamm-post-view-level12/profile.json
```

## Limitations

- Criterion uses synthetic balanced trees, sample size 10, a 500 ms warm-up,
  and a one-second measurement interval. Real genealogies can have different
  depth, width, overlap, and relationship fan-out.
- The report compares designated slope point estimates from separate short
  runs. Criterion's retained statistical base may refer to a later repeated
  run rather than the published loop-1 point estimate.
- The largest parse benchmark run was noisy. No parse/validation change is
  attributed to the view-only patch.
- Direct CLI times are rounded to hundredths of a second, use a warm
  filesystem and already-built binary, and are not a calibrated latency
  distribution. Exact ratios are intentionally not claimed.
- Profiles with 12 and 53 samples cannot support a trustworthy function-level
  percentage breakdown. Fixed-interval sample counts are not themselves CPU
  percentages.
- The saved profiles depend on local symbol resolution against the matching
  debug-symbol release binary.
- The level-12 comparison has a retained post-loop-1 result but no original
  pre-optimization profile.
- Native arm64 measurements do not establish WASM constants, browser worker
  transfer cost, main-thread responsiveness, React cost, or rendering time.
- Hash operations are average-time claims and use more transient memory than
  ordered vectors. Exact peak memory was not measured.

Loop 2 satisfies the native 80-percent objective. Further optimization should
wait for browser evidence that identifies a new dominant stage.
