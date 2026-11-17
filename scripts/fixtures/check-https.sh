#!/usr/bin/env bash
# Opt-in TLS playback proof. Uses a disposable simulator, never the developer's keychain.
set -euo pipefail
cd "$(dirname "$0")/../.."
mkdir -p .agent-work
tls_dir=$(mktemp -d "$PWD/.agent-work/tls-proof.XXXXXX")
tls_device=""
tls_good_pid=""
tls_bad_pid=""
cleanup() {
  set +e
  [[ -z "$tls_good_pid" ]] || kill "$tls_good_pid" 2>/dev/null
  [[ -z "$tls_bad_pid" ]] || kill "$tls_bad_pid" 2>/dev/null
  if [[ -n "$tls_device" ]]; then
    xcrun simctl shutdown "$tls_device" >/dev/null 2>&1
    xcrun simctl delete "$tls_device" >/dev/null 2>&1
  fi
  rm -rf -- "$tls_dir"
}
trap cleanup EXIT
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$tls_dir/ca.key" -out "$tls_dir/ca.pem" -days 2 -subj '/CN=Cue local test root' -addext 'basicConstraints=critical,CA:TRUE' -addext 'keyUsage=critical,keyCertSign,cRLSign' 2>"$tls_dir/openssl.log"
openssl req -newkey rsa:2048 -nodes -keyout "$tls_dir/server.key" -out "$tls_dir/server.csr" -subj '/CN=127.0.0.1' 2>>"$tls_dir/openssl.log"
cat >"$tls_dir/extensions.cnf" <<'EXTENSIONS'
subjectAltName=IP:127.0.0.1
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
EXTENSIONS
openssl x509 -req -in "$tls_dir/server.csr" -CA "$tls_dir/ca.pem" -CAkey "$tls_dir/ca.key" -CAcreateserial -out "$tls_dir/server.pem" -days 2 -extfile "$tls_dir/extensions.cnf" 2>>"$tls_dir/openssl.log"
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$tls_dir/untrusted.key" -out "$tls_dir/untrusted.pem" -days 2 -subj '/CN=127.0.0.1' -addext 'subjectAltName=IP:127.0.0.1' 2>>"$tls_dir/openssl.log"
python3 scripts/fixtures/serve-https.py --cert "$tls_dir/server.pem" --key "$tls_dir/server.key" --port 42425 &
tls_good_pid=$!
python3 scripts/fixtures/serve-https.py --cert "$tls_dir/untrusted.pem" --key "$tls_dir/untrusted.key" --port 42426 &
tls_bad_pid=$!
tls_device=$(xcrun simctl create Cue-HTTPS-Proof com.apple.CoreSimulator.SimDeviceType.Apple-TV-4K-3rd-generation-4K com.apple.CoreSimulator.SimRuntime.tvOS-26-5)
xcrun simctl boot "$tls_device"
xcrun simctl bootstatus "$tls_device" -b
kill -0 "$tls_good_pid" "$tls_bad_pid"
xcrun simctl keychain "$tls_device" add-root-cert "$tls_dir/ca.pem"
curl --fail --silent --cacert "$tls_dir/ca.pem" --range 0-15 https://127.0.0.1:42425/episode01.mp4 --output "$tls_dir/range.bin"
xcodegen generate --spec apple-tv/project.yml
xcodebuild -project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV -configuration Debug -derivedDataPath .agent-work/DerivedData CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM= AppIdentifierPrefix=CueTVSimulator. -destination "platform=tvOS Simulator,id=$tls_device" "-only-testing:SittingRoomTVUITests/${1:-HTTPSPlaybackTests}" test
