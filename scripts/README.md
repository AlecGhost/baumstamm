# Performance profiling scripts

## Whole-program CPU profile

`profile-cpu-samply.sh` profiles a deterministic release build of the
Baumstamm CLI with [`samply`](https://github.com/mstange/samply). The generated
balanced family tree exercises the complete `view` workflow: JSON parsing,
consistency validation, ancestor/descendant traversal, person and relationship
filtering, and result serialization.

Install the pinned profiler version:

```sh
cargo install samply --version 0.12.0 --locked
```

Then run it from the repository root:

```sh
scripts/profile-cpu-samply.sh
```

By default, the fixture, captured CLI output, and `profile.json` are written
to a unique directory under `${TMPDIR:-/tmp}/baumstamm-profile`, keeping
generated artifacts outside tracked source. Choose another location with:

```sh
scripts/profile-cpu-samply.sh --output-dir ./tmp/my-baumstamm-profile
```

The script passes `--save-only`, so samply writes the profile without opening a
browser or starting its local server. Load `profile.json` manually at
[Firefox Profiler](https://profiler.firefox.com/) to inspect its flame graph,
call tree, and timeline. The profile stays local unless you explicitly upload
it.

On Linux, samply needs permission to use performance events. Follow samply's
documented `perf_event_paranoid` setup if recording is rejected.

The default tree has 10 descendant levels, 3,070 people, and 2,047
relationships. To make a shorter validation run or a longer profile, set
`BAUMSTAMM_PROFILE_LEVELS` to a value from 1 through 12:

```sh
BAUMSTAMM_PROFILE_LEVELS=8 scripts/profile-cpu-samply.sh
```

Run `scripts/profile-cpu-samply.sh --help` for the concise command reference.
The workload is deterministic, while statistical sampling means individual
sample counts can vary slightly.
