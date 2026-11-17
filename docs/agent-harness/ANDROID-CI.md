# Android CI Companion peer setup

The four `CompanionSessionTest` cases launch `tests/companion_peer.py` through a repository-local Python virtual environment. The peer listens only on loopback and uses disposable identities; it does not connect to a physical Apple TV.

The Build workflow must create `.venv`, install the existing `bridge/requirements.txt`, and pass `COUCHTV_TEST_PYTHON=$GITHUB_WORKSPACE/.venv/bin/python` to the Gradle test step. The test derives the repository path from that interpreter location, so an arbitrary system Python path is insufficient. No production code or test assertions need changing.

For local execution, with Java 21, Android SDK 34 and the remote web assets built as required by the Android project:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r bridge/requirements.txt
COUCHTV_TEST_PYTHON="$PWD/.venv/bin/python" tv-home/android/gradlew -p on-tv/android --no-daemon --console=plain testDebugUnitTest
```

## Diagnosis and handoff

The main run [44142386043](https://github.com/supagroova/cue-tv/actions/runs/44142386043) failed with 128 tests, four failures and one skipped test. Its quiet log did not preserve individual failures. The retained XML report from [44059320515](https://github.com/supagroova/cue-tv/actions/runs/44059320515), whose Companion test source is identical to main, identifies all four failures as `IllegalStateException: Set COUCHTV_TEST_PYTHON to the project's .venv/bin/python for interoperability tests`.

The failures cover identity/launch with interleaved events, the awake peer without an extra Back key, rejection of changed identity, and rejection of an incorrect signature. The existing fix from `f668779` is applied independently of the product PR stack. The workflow now retains XML and HTML reports even when tests fail, uses plain Gradle output, and preserves all test assertions and selection. Existing bridge dependency declarations remain unchanged.

The separate fix PR targets main; its current CI run is the authoritative post-fix result. The local environment has no Android SDK, so no local Android-suite pass is claimed. This verifies the local protocol peer, not physical-device interoperability. Leave the PR for user review and merge.
