# Quarry task runner. `just check` runs exactly what CI runs.

set shell := ["bash", "-euo", "pipefail", "-c"]

coverage_report := "target/llvm-cov/coverage.json"
core_packages := "-p quarry-engine -p quarry-challenges -p quarry-store"

default: check

# Install every tool the gates need, at pinned versions, and the git hooks.
setup:
    cargo install --locked cargo-nextest@0.9.151 cargo-llvm-cov@0.9.1 cargo-deny@0.20.2 cargo-machete@0.9.2 typos-cli@1.52.0 cargo-mutants@27.1.0 rust-code-analysis-cli@0.0.25
    brew install just lefthook
    if command -v asdf >/dev/null; then asdf reshim rust; fi
    lefthook install

# Every gate, in order from fastest to slowest.
check: fmt lint quality deny machete typos complexity coverage

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Workspace rules: module names, lint inheritance, dependency direction, exceptions.
quality:
    cargo xtask check-names
    cargo xtask check-manifests
    cargo xtask check-deps
    cargo xtask check-exceptions

deny:
    cargo deny check

machete:
    cargo machete

typos:
    typos

complexity:
    cargo xtask complexity

test:
    cargo nextest run --workspace

# Runs the tests under coverage, then enforces the per-crate thresholds.
coverage:
    mkdir -p target/llvm-cov
    cargo llvm-cov nextest --workspace --ignore-filename-regex '(^|/)main\.rs$' --json --summary-only --output-path {{coverage_report}}
    cargo xtask coverage-gate --report {{coverage_report}}

# Mutation testing of the core crates. Pass a diff file to test only changed code.
# cargo-mutants exits 2 when mutants are missed; the score gate decides pass/fail.
# With nothing to mutate it writes no outcomes.json, so there is nothing to gate.
mutants diff="":
    rm -rf mutants.out mutants.out.old
    status=0; cargo mutants {{core_packages}} {{ if diff != "" { "--in-diff " + diff } else { "" } }} || status=$?; \
    if [ "$status" -ne 0 ] && [ "$status" -ne 2 ]; then exit "$status"; fi
    if [ -f mutants.out/outcomes.json ]; then cargo xtask mutation-gate --outcomes mutants.out/outcomes.json; else echo "No mutants under the active filters; nothing to gate."; fi
