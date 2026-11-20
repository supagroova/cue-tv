# Build the standalone Apple TV app locally

Run these commands from a checkout of `supagroova/cue-tv` on an Apple Silicon Mac.
The standalone target embeds Core, torrent transfer and playback. It no longer
requires the upstream Android service, DietPi or a separately installed player.
That architecture has simulator evidence; independent physical-device use is still
unqualified. See [phase 6](PHASE-6.md) and [release preparation](PHASE-7.md).

## Prerequisites and source

Use the pinned build environment: Xcode 26.6, Rust 1.100.0 and XcodeGen 2.46.0.
The local SDK/simulator evidence is tvOS 26.5, not verification of the user's
reported tvOS 27.2 device. The exact tool installation/checksum procedure is in
[setup-apple](../../.github/actions/setup-apple/action.yml). Select Xcode and make
`rustup`, `cargo`, `xcodegen`, Python 3 and Make available in your shell.

```sh
git clone --recurse-submodules https://github.com/supagroova/cue-tv.git
cd cue-tv
git checkout YOUR_REVIEWED_COMMIT
git submodule update --init --recursive
make doctor
```

Replace `YOUR_REVIEWED_COMMIT` with the exact reviewed revision; do not use an
unrecorded moving branch for release evidence. Dependencies are pinned in the
project specification, Cargo manifests/lockfiles and vendor/submodule state.
Do not update them implicitly to resolve a build failure.

## Build and bounded simulator checks

```sh
make check
make apple-build
xcrun simctl list devices available
```

`make apple-build` generates `apple-tv/SittingRoomTV.xcodeproj` and builds the
**Debug**, unsigned device app under `.agent-work/DerivedData`. It is compilation
evidence, not a signed installable release. The helper creates missing ignored
signing/config placeholders but preserves existing ones.

The native integration tests require the owned loopback peer fixture. Start this
in a separate terminal, from the same checkout, and leave it running during tests:

```sh
cargo +1.100.0 run --locked --manifest-path native/cue-torrent/Cargo.toml --example seed
```

Then use an actual available simulator UDID:

```sh
make apple-ci-test DESTINATION='platform=tvOS Simulator,id=YOUR_SIMULATOR_UDID'
```

This runs unit/native integration tests and the short playback UI suite in one
Xcode invocation. Stop the fixture process you started afterward. Do not run
concurrent Xcode builds against the same DerivedData directory. Longer account,
full-journey and sustained-playback runners remain opt-in; use their phase reports
for server ownership and fixture prerequisites.

## Signing and physical installation

Device installation has not been verified here. An unsigned app cannot be installed
as a normal sideloaded app. Pair the Apple TV in Xcode, add the developer account,
and configure the local ignored `apple-tv/Config/Signing.mine.xcconfig` using
[the example](../../apple-tv/Config/Signing.example.xcconfig). Preserve existing
team and bundle identity for upgrades; do not commit credentials or provisioning
material. Open the generated Xcode project, select the Apple TV destination and
review Signing & Capabilities before building/installing.

Do not describe this as a release procedure completed successfully: Release
transport configuration, actual signing/install, detached launch and same-identity
upgrade are open gates in phases 6–7. The [phase 7 checklist](PHASE-7.md) records
what must accompany a candidate before handover.

## Historical baseline

On 16 November 2026 the extracted upstream Apple target compiled unsigned, five
existing tests passed, and its Android-service connection screen was displayed.
That historical experiment did not prove standalone media playback. Its source
provenance remains in [SOURCE-MANIFEST.json](SOURCE-MANIFEST.json); that file is an
upstream snapshot manifest, **not** a manifest for the current combined binary.
No physical installation claim follows from either that experiment or later
simulator tests.
