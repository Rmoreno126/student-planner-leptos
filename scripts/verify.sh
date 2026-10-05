#!/usr/bin/env bash
# Runs the whole check loop and saves the full output:  ./scripts/verify.sh
cd "$(dirname "$0")/.."
export DATABASE_URL=postgres://postgres:dev@localhost:5432/planner
mkdir -p /tmp/target
steps=(
  "cargo fmt --all"
  "cargo check --features ssr"
  "cargo check --lib --features hydrate --target wasm32-unknown-unknown"
  "cargo clippy --features ssr -- -D warnings"
  "cargo test --lib --features ssr"
  "cargo test --test db_tasks --features ssr"
)
: > verify.log
failed=0
for step in "${steps[@]}"; do
  echo "== $step" | tee -a verify.log
  if ! bash -c "$step" >> verify.log 2>&1; then
    echo "FAILED: $step" | tee -a verify.log
    failed=1
    break
  fi
done
grep -E 'test result' verify.log
if [ "$failed" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "--- first error (full output in verify.log) ---"
  grep -m1 -A8 '^error' verify.log
fi
