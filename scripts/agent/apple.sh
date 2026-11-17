#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
action=${1:-build}
case "$action" in build|test|ui-test) ;; *) echo "usage: apple.sh build|test|ui-test" >&2; exit 2;; esac
command -v xcodegen >/dev/null || { echo 'Install XcodeGen before generating the project.' >&2; exit 2; }
command -v xcodebuild >/dev/null || { echo 'Select a supported Xcode installation first.' >&2; exit 2; }
if [[ "$action" != build && -z "${DESTINATION:-}" ]]; then
  echo 'Set DESTINATION to an explicit tvOS simulator destination.' >&2; exit 2
fi
mkdir -p .agent-work
if [[ ! -f apple-tv/Config/Signing.mine.xcconfig ]]; then
  printf 'DEVELOPMENT_TEAM =\n' > apple-tv/Config/Signing.mine.xcconfig
fi
if [[ ! -f apple-tv/TVConfig.plist ]]; then
  cp apple-tv/TVConfig.example.plist apple-tv/TVConfig.plist
fi
xcodegen generate --spec apple-tv/project.yml
args=(-project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV -configuration Debug -derivedDataPath .agent-work/DerivedData CODE_SIGNING_ALLOWED=NO)
if [[ "$action" == build ]]; then
  xcodebuild "${args[@]}" -sdk appletvos -destination 'generic/platform=tvOS' build
else
  target=SittingRoomTVTests
  [[ "$action" == ui-test ]] && target=SittingRoomTVUITests
  xcodebuild "${args[@]}" -destination "$DESTINATION" -only-testing:"$target" test
fi
