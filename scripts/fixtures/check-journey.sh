#!/usr/bin/env bash
# Opt-in real Core/add-on navigation proof, kept out of short automatic CI.
set -euo pipefail
cd "$(dirname "$0")/../.."
journey_pid=""
peer_pid=""
journey_dir=$(mktemp -d "$PWD/.agent-work/journey.XXXXXX")
cleanup() {
  [[ -z "$journey_pid" ]] || kill "$journey_pid" 2>/dev/null || true
  [[ -z "$peer_pid" ]] || kill "$peer_pid" 2>/dev/null || true
  [[ -z "$peer_pid" ]] || wait "$peer_pid" 2>/dev/null || true
  rm -rf -- "$journey_dir"
}
trap cleanup EXIT
cargo +1.100.0 build --locked --manifest-path native/cue-torrent/Cargo.toml --example journey_seed --target-dir .agent-work/native-target
.agent-work/native-target/debug/examples/journey_seed "$journey_dir" >"$journey_dir/peer.log" 2>&1 &
peer_pid=$!
for _ in {1..50}; do
  kill -0 "$peer_pid"
  if rg -q 'Journey peer ready' "$journey_dir/peer.log"; then break; fi
  sleep 0.1
done
rg -q 'Journey peer ready' "$journey_dir/peer.log"
python3 scripts/fixtures/serve-addon.py &
journey_pid=$!
sleep 0.2
kill -0 "$journey_pid"
curl --fail --silent http://127.0.0.1:42427/manifest.json >/dev/null
xcodegen generate --spec apple-tv/project.yml
xcodebuild -project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV -configuration Debug -derivedDataPath .agent-work/DerivedData CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM= AppIdentifierPrefix=CueTVSimulator. -destination "${1:-platform=tvOS Simulator,name=Apple TV 4K (3rd generation)}" "-only-testing:SittingRoomTVUITests/${2:-StandaloneJourneyTests}" test
