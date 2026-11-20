#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
case "${PLATFORM_NAME:-appletvsimulator}" in
  appletvos) target=aarch64-apple-tvos ;;
  appletvsimulator) target=aarch64-apple-tvos-sim ;;
  *) echo 'Unsupported Apple platform' >&2; exit 2 ;;
esac
export CARGO_TARGET_DIR="$PWD/.agent-work/native-target"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
cargo +1.100.0 build --locked --manifest-path native/cue-torrent/Cargo.toml --target "$target"
