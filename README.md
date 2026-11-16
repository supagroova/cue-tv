# Cue TV

A custom TV home and iPhone remote for Stremio and YouTube, with a native Apple TV browsing app.

Built from [Seagey/couch-tv](https://github.com/Seagey/couch-tv), retaining its MIT licence and upstream source. This is a private development repository for collaboration. The upstream documentation is retained in [docs/UPSTREAM-README.md](docs/UPSTREAM-README.md).

## Current architecture

- `on-tv/android/`: Android TV app, local API, pairing, Stremio integration and Apple TV control bridge.
- `remote/`: React phone interface served by the Android TV.
- `iphone/`: native iPhone shell, wake, Keychain pairing and separate YouTube website sessions.
- `apple-tv/`: native tvOS browsing interface, currently connected to the Android API.
- `tv-home/web/`: custom Android home and browsing interface.

Stremio browsing now stays inside Cue TV, including seasons, episodes, exact Real-Debrid Torrentio source selection and internal Android/tvOS players. YouTube continues to open its own app on the selected platform. Stremio Core was consulted for protocol compatibility; its Rust engine is not embedded. No always-on Mac is required by the current hybrid installation.

## Current update

- Visible app names are **Cue TV**, preserving existing bundle IDs and household pairings.
- The remote supports held Select without text selection. Apple TV has a separate TV button: tap for the system TV action, hold for Control Centre. Android's existing raw TV key remains available.
- The iPhone YouTube panel has compact controls and separate named website feeds. Sending a video requires an explicit **Play on TV** action. Build 5 is installed on the owner's phone; actual personalised layout and previews need a fresh household check.
- Netflix, Prime Video and Apple TV+ have separate **Trending**, **Popular** and **New** film/show shelves in the configured country. These are provider-labelled discovery catalogues, not private viewing figures. Stream providers remain Real-Debrid Torrentio only.
- Cue stores its own playback progress under the current Stremio account fingerprint and merges it with existing history. It does not overwrite the external Stremio account.

Android release, lint and 135 unit checks completed with no failures and one optional network skip. Remote/browser checks cover deliberate hold, cancellation, platform routing, separate service rankings and exact episode/source selection. Native tvOS tests played a generated Matroska fixture, recorded real progress, paused and returned to the same source chooser. The Android internal player has played a real selected source on hardware. The tvOS update is installed, but the latest real-device phone-to-player handoff timed out and remains unresolved. See [delivery evidence and limits](docs/CUE-TV-DELIVERY.md) and [playback architecture](docs/CUE-TV-PLAYBACK.md).

## Deployment modes

The current Android-hosted installation works with Android TV and the existing Apple TV hybrid. **Standalone Apple TV only remains unfinished.** It needs independent native phone discovery/pairing, remote control and Stremio browsing, without depending on an Android host. A platform picker does not provide that capability. Preserve existing household connections when implementing the three complete setup modes.

## Local development

Use Python 3.11+, a supported Node version, JDK 21/Android SDK 36 for Android (minimum Android 6; target SDK 34), and Xcode/XcodeGen for Apple targets. See the target READMEs for existing build commands.

```sh
cd remote
npm ci
npm run build
```

Phone-interface fixture, with no TV commands:

```sh
COUCHTV_FIXTURE_PORT=5400 python3 tests/hybrid_fixture.py
```

Before generating Apple projects:

```sh
cp iphone/TVConfig.example.plist iphone/TVConfig.plist
cp apple-tv/TVConfig.example.plist apple-tv/TVConfig.plist
cp apple-tv/Config/Signing.example.xcconfig apple-tv/Config/Signing.mine.xcconfig
```

The examples contain dummy network values. Replace them locally for your own TV. These local files are ignored. Run `xcodegen generate` inside the relevant Apple target directory. The shared project leaves DEVELOPMENT_TEAM empty; choose your own team for a physical-device build. If using a different Apple team, use your own bundle identifiers consistently for the app, extension and extension launch identifier. Simulator checks do not require household signing material.

## Collaboration

Use feature branches and pull requests. Keep `main` buildable. Source changes do not authorise installation or commands on anyone's TV. Never replace a household install with a differently signed debug APK. In-place updates need the owner's local signing identity and the existing guarded installer.

No device pairings, account tokens, Google client credentials, Real-Debrid configuration, signing keys, household configuration, recordings or build caches are included here. Each installation pairs and signs into its own accounts.

Before committing, install the official `gitleaks` tool and run:

```sh
bash scripts/privacy-scan.sh
```

The generic scanner has exact content-hash exceptions for reviewed synthetic test inputs. Changing those inputs invalidates their exception. Secret detection and forbidden-file checks are never filtered. See [docs/SHARING.md](docs/SHARING.md).
