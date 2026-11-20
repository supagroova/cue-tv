#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
case "${PLATFORM_NAME:-appletvsimulator}" in
  appletvos) target=aarch64-apple-tvos ;;
  appletvsimulator) target=aarch64-apple-tvos-sim ;;
  *) echo 'Unsupported Apple platform' >&2; exit 2 ;;
esac
export CARGO_TARGET_DIR="$PWD/.agent-work/native-target"
if [[ "${CI:-}" == true && "$target" == aarch64-apple-tvos-sim && -n "${CUE_TORRENT_CACHE_KEY:-}" ]]; then
  [[ "$CUE_TORRENT_CACHE_KEY" == "$(python3 scripts/native/artifact-cache.py key torrent)" ]] || { echo 'Native cache sources changed'; exit 1; }
  [[ -s "$CARGO_TARGET_DIR/$target/debug/libcue_torrent.a" ]] || { echo 'Native cache output missing'; exit 1; }
  echo 'Using verified exact-source simulator torrent artifacts'
  exit 0
fi
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
cargo +1.100.0 build --locked --manifest-path native/cue-torrent/Cargo.toml --target "$target"
