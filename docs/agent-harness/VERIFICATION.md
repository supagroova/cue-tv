# Harness verification

Verified locally on 17 November 2026. These results cover the harness and existing app baseline, not standalone torrent implementation.

| Check | Result |
|---|---|
| `make check` | Passed: structure/link/alias checks and 10 Python behavior tests |
| skill-creator `quick_validate.py` on every canonical skill | All 11 passed using an isolated local Python environment with PyYAML |
| `bash -n scripts/agent/apple.sh scripts/agent/install-hooks.sh .githooks/pre-commit` | Passed |
| `make apple-build` | Unsigned tvOS build passed with Xcode 26.6 (17F113) and XcodeGen 2.46.0 |
| `make apple-test DESTINATION='platform=tvOS Simulator,id=…'` | Five Swift Testing tests passed on Apple TV 4K third-generation simulator, tvOS 26.5 |
| Real Git pre-commit negative fixture | Synthetic signing file blocked; allowed source staging passes the path guard |
| Hook activation | Checkout-local `core.hooksPath=.githooks`; does not install provider command hooks |
| UI tests / physical Apple TV / torrent playback / signing | Not run in this task |

XcodeGen was supplied through a command-local PATH from the previously downloaded tool; no global install was performed. `make doctor` correctly reports it absent from the default PATH. Rust is not installed and no Rust gate is claimed. The Swift Testing run reports five tests separately from XCTest's empty suite; the empty suite was not counted as proof.

Local logs are under ignored `.agent-work/apple-build.log`, `.agent-work/apple-test.log` and DerivedData test results. Failed early harness setup checks were corrected before the passing runs. The evidence helper also retains command failure and retry logs in its own fixtures. Remote CI execution is reported in the final handover only after observing the actual run.

Review performed inline: task scope, upstream setup preservation, skill discovery links, negative guard behavior, cancellation-independent evidence, safe local configuration, CI permissions and test-surface limits. No independent subagent review is claimed. No application source or dependency was changed.
