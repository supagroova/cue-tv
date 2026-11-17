#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
expected=2c89236b47dfcd5e844c1dfae5737164cb616fbf
actual=$(git -C native/stremio-core rev-parse HEAD)
[[ "$actual" == "$expected" ]] || { echo 'Stremio Core submodule revision mismatch'; exit 1; }
git -C native/stremio-core diff HEAD --quiet || { echo 'Stremio Core submodule has unreviewed changes'; exit 1; }
case "${PLATFORM_NAME:-appletvsimulator}" in
  appletvos) target=aarch64-apple-tvos ;;
  appletvsimulator) target=aarch64-apple-tvos-sim ;;
  *) echo 'Unsupported Apple platform' >&2; exit 2 ;;
esac
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$PWD/.agent-work/native-target"}
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
cargo +1.100.0 build --locked --manifest-path native/cue-core/Cargo.toml --target "$target"
