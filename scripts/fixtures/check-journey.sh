#!/usr/bin/env bash
# Opt-in real Core/add-on navigation proof, kept out of short automatic CI.
set -euo pipefail
cd "$(dirname "$0")/../.."
journey_pid=""
cleanup() { [[ -z "$journey_pid" ]] || kill "$journey_pid" 2>/dev/null || true; }
trap cleanup EXIT
python3 scripts/fixtures/serve-addon.py &
journey_pid=$!
sleep 0.2
kill -0 "$journey_pid"
curl --fail --silent http://127.0.0.1:42427/manifest.json >/dev/null
xcodegen generate --spec apple-tv/project.yml
xcodebuild -project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV -configuration Debug -derivedDataPath .agent-work/DerivedData CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM= AppIdentifierPrefix=CueTVSimulator. -destination "${1:-platform=tvOS Simulator,name=Apple TV 4K (3rd generation)}" -only-testing:SittingRoomTVUITests/StandaloneJourneyTests test
