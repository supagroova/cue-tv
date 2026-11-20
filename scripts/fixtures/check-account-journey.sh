#!/usr/bin/env bash
# Opt-in account UI recovery proof. No live credentials or public account service.
set -euo pipefail
cd "$(dirname "$0")/../.."
account_pid=""
account_case="${2:-all}"
case "$account_case" in
  all) account_test="AccountJourneyTests" ;;
  failed-save) account_test="AccountJourneyTests/testFailedCredentialSaveRetainsPendingLibraryAndCanRetry" ;;
  *) echo "Unknown account journey case: $account_case" >&2; exit 2 ;;
esac
cleanup() {
  [[ -z "$account_pid" ]] || kill "$account_pid" 2>/dev/null || true
  [[ -z "$account_pid" ]] || wait "$account_pid" 2>/dev/null || true
}
trap cleanup EXIT
account_log=$(mktemp "$PWD/.agent-work/account-journey.XXXXXX")
echo "Account fixture log: $account_log"
python3 scripts/fixtures/serve-account.py >"$account_log" 2>&1 &
account_pid=$!
for _ in {1..50}; do
  kill -0 "$account_pid"
  if rg -q "Synthetic account fixture ready" "$account_log"; then break; fi
  sleep 0.1
done
rg -q "Synthetic account fixture ready" "$account_log"
kill -0 "$account_pid"
curl --max-time 2 --fail --silent http://127.0.0.1:42429/probe >/dev/null
xcodegen generate --spec apple-tv/project.yml
xcodebuild -project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV -configuration Debug -derivedDataPath .agent-work/DerivedData CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM= AppIdentifierPrefix=CueTVSimulator. -destination "${1:-platform=tvOS Simulator,name=Apple TV 4K (3rd generation)}" "-only-testing:SittingRoomTVUITests/$account_test" test
python3 - "$account_case" <<'PY'
import json
import sys
from urllib.request import urlopen
with urlopen('http://127.0.0.1:42429/probe', timeout=2) as response:
    observed = json.load(response)
expected = (dict(accepted_logins=3, rejected_logins=0, accepted_uploads=1, rejected_uploads=1)
            if sys.argv[1] == "failed-save" else
            dict(accepted_logins=6, rejected_logins=2, accepted_uploads=2, rejected_uploads=2))
assert observed == expected, (observed, expected)
print('Verified expected account login and pending-library upload counts over real HTTP.')
PY
