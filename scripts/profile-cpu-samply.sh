#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Generate a whole-program CPU profile for Baumstamm's CLI view workflow.

Usage:
  scripts/profile-cpu-samply.sh [--output-dir DIR]
  scripts/profile-cpu-samply.sh --help

Options:
  --output-dir DIR  Store the fixture, CLI output, and profile.json.gz in DIR.
                    By default a unique directory below
                    ${TMPDIR:-/tmp}/baumstamm-profile is used.
  -h, --help        Show this help.

Environment:
  BAUMSTAMM_PROFILE_LEVELS
                    Number of descendant levels in the generated balanced
                    family tree (default: 10; allowed: 1..12).

Prerequisites:
  - Rust and Cargo
  - samply 0.12.0:
    cargo install samply --version 0.12.0 --locked

The deterministic workload runs `baumstamm-cli view` in release mode with
debug symbols. It exercises loading, consistency checks, view traversal,
filtering, and serialization. The resulting profile can be loaded in Firefox
Profiler to inspect flame graphs, call trees, and timelines. Sampling results
themselves can vary slightly between runs.
EOF
}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

output_dir=

while (($# > 0)); do
    case "$1" in
        --output-dir)
            (($# >= 2)) || die "--output-dir requires a directory"
            output_dir=$2
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown argument '$1' (run with --help for usage)"
            ;;
    esac
done

command -v cargo >/dev/null 2>&1 ||
    die "Cargo is required; install Rust from https://rustup.rs/"
command -v samply >/dev/null 2>&1 ||
    die "samply 0.12.0 is required; install it with: cargo install samply --version 0.12.0 --locked"
command -v awk >/dev/null 2>&1 ||
    die "awk is required to generate the deterministic workload"

samply_version=$(samply --version 2>/dev/null) ||
    die "could not determine the installed samply version"
[[ $samply_version == "samply 0.12.0" ]] ||
    die "samply 0.12.0 is required (found: $samply_version); install it with: cargo install samply --version 0.12.0 --locked"

levels=${BAUMSTAMM_PROFILE_LEVELS:-10}
[[ $levels =~ ^[0-9]+$ ]] ||
    die "BAUMSTAMM_PROFILE_LEVELS must be an integer from 1 through 12"
((levels >= 1 && levels <= 12)) ||
    die "BAUMSTAMM_PROFILE_LEVELS must be between 1 and 12"

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)

if [[ -z $output_dir ]]; then
    artifact_base=${TMPDIR:-/tmp}/baumstamm-profile
    run_id=$(date -u +%Y%m%dT%H%M%SZ)-$$
    output_dir=$artifact_base/$run_id
fi

mkdir -p -- "$output_dir"
output_dir=$(CDPATH= cd -- "$output_dir" && pwd)
fixture=$output_dir/workload.json
view_output=$output_dir/view-output.json
profile=$output_dir/profile.json.gz

for artifact in "$fixture" "$view_output" "$profile"; do
    [[ ! -e $artifact ]] ||
        die "refusing to overwrite existing artifact: $artifact"
done

fixture_tmp=$fixture.tmp.$$
cleanup() {
    rm -f -- "$fixture_tmp"
}
trap cleanup EXIT

LC_ALL=C awk -v levels="$levels" '
BEGIN {
    nodes = 1
    width = 1
    for (level = 0; level < levels; level++) {
        width *= 2
        nodes += width
    }
    internal = (nodes - 1) / 2
    persons = nodes + internal

    print "{"
    print "  \"relationships\": ["
    print "    {\"id\":\"0\",\"parents\":[null,null],\"children\":[\"0\"]},"
    for (i = 0; i < internal; i++) {
        spouse = nodes + i
        founding_rel = 1 + (2 * i)
        family_rel = founding_rel + 1
        left_child = (2 * i) + 1
        right_child = left_child + 1

        printf "    {\"id\":\"%X\",\"parents\":[null,null],\"children\":[\"%X\"]},\n",
            founding_rel, spouse
        printf "    {\"id\":\"%X\",\"parents\":[\"%X\",\"%X\"],\"children\":[\"%X\",\"%X\"]}",
            family_rel, i, spouse, left_child, right_child
        if (i + 1 < internal) {
            printf ","
        }
        printf "\n"
    }
    print "  ],"
    print "  \"persons\": ["
    for (i = 0; i < persons; i++) {
        printf "    {\"id\":\"%X\",\"info\":null}", i
        if (i + 1 < persons) {
            printf ","
        }
        printf "\n"
    }
    print "  ]"
    print "}"
}
' > "$fixture_tmp"
mv -- "$fixture_tmp" "$fixture"

printf 'Building release binary with debug symbols\n'
cd -- "$repo_root"
CARGO_PROFILE_RELEASE_DEBUG=2 \
    cargo build --release --package baumstamm-cli --bin baumstamm-cli

printf 'Profiling deterministic level-%s workload into %s\n' "$levels" "$output_dir"
samply record --save-only --output "$profile" \
    "$repo_root/target/release/baumstamm-cli" \
    "$fixture" view 0 \
    --show-partners \
    --show-siblings \
    --show-ancestor-siblings \
    --show-partner-siblings \
    > "$view_output"

[[ -s $profile ]] || die "samply did not produce $profile"
[[ -s $view_output ]] || die "the profiled CLI did not produce view output"

printf 'Profile:    %s\n' "$profile"
printf 'Fixture:    %s\n' "$fixture"
printf 'CLI output: %s\n' "$view_output"
