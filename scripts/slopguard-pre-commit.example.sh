#!/usr/bin/env bash
# Example pre-commit / CI gate: fails if the engine reports any issue.
# Copy to your repo root, chmod +x, point ENGINE to a release binary.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE="${SLOPGUARD_ENGINE:-$ROOT/engine/target/release/slopguard-engine}"
if [[ ! -x "$ENGINE" ]]; then
  echo "slopguard: engine not found at $ENGINE (build with: cd engine && cargo build --release)"
  exit 2
fi
exec "$ENGINE" scan "$ROOT" --max-files 2000
