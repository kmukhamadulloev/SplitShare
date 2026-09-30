#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm run build --prefix web
cargo build --locked -p splitshare
pids=()
cleanup() {
  trap - EXIT INT TERM
  kill "${pids[@]}" 2>/dev/null || true
  wait "${pids[@]}" 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
./target/debug/splitshare --dev "$@" &
pids+=("$!")
(cd web && exec npm run dev -- --strictPort) &
pids+=("$!")
wait -n "${pids[@]}"
