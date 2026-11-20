# Phase 7 — personal release and handover

Status: handover implementation in progress; **no qualified signed release**. Branch
`codex/tvos-phase-7` stacks on phase 6. Neither this checklist nor green CI waives
[phase 6 qualification](PHASE-6.md) or unresolved earlier gates.

## Candidate record and reproducibility

For each candidate, retain a sanitized record with the following artifacts. All
are pending for a final signed candidate:

| Artifact | Required evidence |
| --- | --- |
| Source identity | Exact Git commit, submodule commits, clean/dirty tree status and any patch needed to reproduce it |
| Dependency manifest | Exact Cargo lockfiles, Swift package revision/resolution, vendor patches and checksums of linked native archives/frameworks |
| Build environment | Actual Xcode, SDK, Rust and XcodeGen versions; architecture, build configuration and exact commands |
| Build output | App/binary checksum, bundle/build version, linked libraries and resource inventory; label unsigned versus signed outputs |
| License/source materials | Notices matching the libraries actually linked, source revisions, patches and build materials needed by their licenses; do not substitute a generic dependency list |
| Signing/install | Nonsecret team/bundle/profile identifiers and expiration from the actual candidate, successful install and debugger-detached launch on the actual device |
| Qualification | All phase 6 cases and inherited gates with raw evidence links and retained failures |
| Upgrade/removal | Same-identity upgrade observations, state preservation and tested removal/credential behavior |

The existing `SOURCE-MANIFEST.json` is provenance for the 16 November upstream
snapshot, not the release manifest. The [dependency inventory](DEPENDENCIES.md)
is a starting point; the final binary must be inspected before claiming its notices
are complete. Reproducibility means a recorded clean build can be repeated; it
does not mean signed binaries are promised to be byte-identical.

[LOCAL-BUILD.md](LOCAL-BUILD.md) documents current verified build entry points.
`make apple-build` is explicitly Debug/unsigned. `make apple-release-build` now
provides a locally verified unsigned Xcode Release build and packaged ATS audit;
native archives still use their existing Cargo development profile. A signed
artifact and physical Release runtime are not yet qualified. Do not relabel either
unsigned output as an installable personal-release package.
Keep signing secrets, personal settings, provisioning profiles and private account
information outside Git and shared reports.

Release configuration excludes the 17 current Playback Lab media/torrent/manifest
resources. CI inspects the packaged app and fails if any fixture resource leaks
back in; Debug retains them for its existing playback tests. The configuration
uses Apple's [excluded-source build setting](https://developer.apple.com/documentation/xcode/build-settings-reference).
This packaging check does not qualify the remaining binary notices or device runtime.

## Unsigned candidate inventory

After initializing local Apple configuration with the normal build command, commit
all source changes and run (with the pinned toolchain on PATH):

```sh
python3 scripts/agent/release-record.py --output .agent-work/release-candidate-001
```

Use a new output directory and do not run another build concurrently. The recorder
requires a clean checkout and pinned submodules, runs the unsigned Release build,
retains its log/receipt, and refuses to issue a manifest if source or existing private
configuration changes. It copies both Cargo lockfiles and Swift's resolved package
file, hashes native archives and every packaged app file, and records tool versions,
bundle versions and linked libraries. Signed or unexpectedly symlinked artifacts
require separate handling rather than silently receiving an unsigned inventory.

Keep this evidence local: private configuration is hashed rather than copied.
The recorder identifies an incremental unsigned build; independent clean-rebuild
repeatability, complete notices/source materials and physical qualification remain
separate gates. It does not replace the historical source snapshot manifest.

Local candidate `release-candidate-001` was recorded successfully at clean commit
`7ee7493c06fcd47b38f36b8540ff05aeb7dec329`: 7 app files, 2 native archives and
3 dependency resolution files. All app checksums were independently rechecked.
The evidence stays under `.agent-work/release-candidate-001`; it identifies this
unsigned incremental candidate, not later commits or an independently reproduced
clean build. Its task receipt is `1651cc4f10454521a5ca5abb91c250ce`.

## First installation and upgrade protocol

1. Complete the phase 6 release audit and select a fully reviewed commit. Record
   candidate identity, dependencies and qualification artifacts before installation.
2. Pair/sign/install through the user's Xcode account and verify the resulting
   device identity and actual OS. Physical pairing remains deferred at present.
3. Launch with the debugger detached. Complete a permitted guest and connected
   Stremio journey, direct peer playback, pause/seek/Back, library edit and resume.
   Confirm no Android, Pi, external player or development-Mac runtime dependency.
4. Preserve the previous known-good signed artifact while its profile remains valid.
   Install the candidate using the same team/bundle identity without deleting the
   app first. Confirm the expected account, library and resume state survive.
5. Record renewal using the actual profile's expiration; do not promise a fixed
   signing lifetime. Re-sign/install with the same identity and repeat detached
   launch and state checks. Changing bundle/team identity is not an upgrade test.
6. Do not promise downgrade support without a demonstrated compatible persistence
   schema and previous-build test. A retained old binary alone does not prove it
   can safely read newer account/library state.

## User handover content

The final app/help and handover must explain the tested behavior, including:

- How to browse permitted Stremio content, choose the intended episode/source and
  resume or restart. Direct torrents are required; debrid is not a v1 prerequisite.
- Disposable media-cache limits/reserve and how to recover from low space or cache
  purge. Current 4 GiB/1 GiB policies remain provisional pending device measurement.
- Peer upload behavior during an active torrent session, and how stopping playback
  ends the owned session. Verify this against the final engine configuration; do
  not promise that download-only operation or background seeding is implemented.
- Account sign-out and recovery from rejected credentials or failed synchronization.
  Failed cleanup must be retried before changing account identity.
- Only media formats, HDR/audio paths and accessibility flows actually tested on
  the target device. Framework capability lists are not product qualification.

Home now provides a Help screen covering watch/resume, the current cache limit
and reserve, peer sharing and account recovery. Its opt-in simulator journey
verifies remote navigation, displayed sections and restored Home focus; visual
review confirms readable text and an opaque background. Physical readability,
VoiceOver and final device-qualified guidance remain unverified.

## Removal and troubleshooting protocol

Before removal, stop playback and use the app's sign-out flow for connected
accounts. Verify cleanup completes before removing the app. Do not assume deleting
an app erases its Keychain entries; removal/credential behavior must be measured
on the target. Private viewing data must not be copied into diagnostic reports.

For a failed build, retain the failing command/log and compare toolchain/dependency
pins before making changes. For installation/launch failure, inspect the actual
signing/profile/device compatibility instead of changing bundle identity blindly.
For playback failure, distinguish discovery, transfer, decode, storage and account
errors. Retain sanitized local counters and reproduction conditions; never upload
raw tokens, configured tracker URLs/passkeys or private library snapshots.

## Exit status

Signed install, renewal, same-identity upgrade, removal behavior, final binary
notices/source manifest, physical help/accessibility review and independent
playback remain unverified or unfinished. The user must ultimately be able to
launch, watch, resume and recover without developer assistance. Until the original
plan's evidence supports that outcome, this phase and the stack are not merge-ready.
