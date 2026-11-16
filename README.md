# Cue TV

A custom TV home and iPhone remote for Stremio and YouTube, with a native Apple TV browsing app.

Built from [Seagey/couch-tv](https://github.com/Seagey/couch-tv), retaining its MIT licence and upstream source. This is a private development repository for collaboration. The upstream documentation is retained in [docs/UPSTREAM-README.md](docs/UPSTREAM-README.md).

## Current architecture

- `on-tv/android/`: Android TV app, local API, pairing, Stremio integration and Apple TV control bridge.
- `remote/`: React phone interface served by the Android TV.
- `iphone/`: native iPhone shell, wake, Keychain pairing and separate YouTube website sessions.
- `apple-tv/`: native tvOS browsing interface, currently connected to the Android API.
- `tv-home/web/`: custom Android home and browsing interface.

Playback uses the installed official Stremio and YouTube apps. Stremio Core was consulted for its playback-link protocol; its Rust engine is not embedded in the custom interface. No always-on Mac is required by the current installation.

## Requested work

1. Rename the visible apps and tvOS interface to **Cue TV**, retaining existing bundle IDs, signing identity and pairings.
2. Stop YouTube feed previews playing automatically, improve the compact browsing layout and require an explicit Play on TV action.
3. Support **Android TV only**, **Apple TV only**, and **Both**. Android currently hosts the service even when Apple TV is selected. Apple TV-only is not implemented yet and requires an independent connection and browsing path.

The imported native iPhone baseline is build 4. Twenty-one unit checks and three UI checks passed. Real signed-in Home recommendations have been confirmed by the owner. Automatic previews and the embedded layout are known issues being addressed; a full recommendation-to-TV playback acceptance check remains separate.

## Local development

Use Python 3.11+, a supported Node version, JDK 21/Android SDK for Android, and Xcode/XcodeGen for Apple targets. See the target READMEs for existing build commands.

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
