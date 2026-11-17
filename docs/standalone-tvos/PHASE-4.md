# Phase 4 — standalone Stremio journey

Status: in progress; depends on unfinished Phase 3. This is not an exit-gate claim.

The first slice adds `TorrentPlaybackView`, accepting a prepared Core source identity. It offers explicit original-index file selection with pagination, in-app VLC rendering, pause/resume, relative seeking, audio selection and subtitle cycling/off. Decoder shutdown runs before native/cache retirement; callback identity rejects retired generations. Backgrounding and Close stop the source.

Guest entry and basic catalog/details/episode/source navigation are now connected. Account sign-in, search, add-on subtitles, focus restoration and full journey validation remain outstanding. Physical playback is unverified.

Verification: unsigned device build passes (`phase-4-ui/a59869c0a1774927885d58cb079db89b`); simulator unit/integration suite passes (`phase-4-ui/249b0a3859314138a3539b685a7f516d`). Those establish compilation and existing integration contracts, not visual/focus acceptance of the new screen. The existing lab UI suite passed on the shared renderer before the controller extraction; new screen UI coverage remains required.

## Guest entry and native browsing

The default app entry now creates the persistent native guest Core service. Basic catalog/title/detail/episode/source windows and add-on preview/install use revision-checked Core commands; selecting a supported torrent opens the source playback screen. Android pairing/remote observation are no longer started by the app entry point. Unsupported non-torrent sources are explicitly disabled in this initial slice; compatible HTTPS remains an outstanding requirement.

The model integration test proves real Core startup, snapshot updates, a language action and joined shutdown. Simulator suite passes (`phase-4-ui/b413d2ac2da049d5b55e7f13bf4f4753`). Visual inspection of the actual simulator guest entry caught white-on-white inherited tint; removing the tint restored readable catalog buttons. Screenshot evidence remains local at `.agent-work/standalone-guest-fixed.png`. Full navigation/focus and player-screen UI tests are still required. Catalog labels also include media kind to distinguish same-name film/series catalogs.

Latest unsigned device build passes: `phase-4-ui/9eda0cfd6157432ea364fddb7e77f16e`.

## Guest lifecycle regression and UI smoke

A regression test reproduced reopening the guest view during prior Core shutdown: the old `running` guard could drop the new run. The model now cancels and joins the previous polling/session task before starting its successor, with generation checks and cancellation propagation. The test waits for a distinct ready native session ID before sending a language action, rather than assuming a fixed startup delay. Red evidence `phase-4-ui/d1bfe7b12ac8499299bd9e6ada69d53f`; final 32-test unit/integration suite green `phase-4-ui/d2a32ee11e6f494bbaa7206b83dc84b9`.

The new guest-entry UI check is in the automatic short UI suite: it verifies guest mode, distinct movie/series catalog buttons and absence of Android pairing fields, and retains a screenshot. All 11 UI tests passed in `phase-4-ui/6a82231853d14d5e9ee4e29e74c285ba`; that combined invocation was overall red because the unit restart test still used its premature fixed delay, subsequently corrected and rerun as above. No claim that the combined run was green.

## Stale action isolation

A blocked action fixture reproduced an error from a retired Core generation overwriting the new guest state (`phase-4-ui/97fa192633304ab9bfbc559cf388d11a`). Action errors and prepared torrent replies now check the captured generation before publishing; cancelled prepare callers also discard results. The final 33-test simulator suite passes (`phase-4-ui/b7df2960077949ac97a31dd9b809c7eb`). These checks cover restart boundaries; cancellation of obsolete search requests and the complete navigation journey remain outstanding.

## Catalog-scoped search

The catalog screen now accepts a search query and submits it only when the user selects a declared search-capable catalog. Unsupported catalogs are disabled while a query is entered; empty input preserves ordinary browsing. This is explicit per-catalog search, not cross-provider aggregation or live keystroke search. Simulator suite passes (`phase-4-ui/f0bae05e5259456b9feaaa94c7ed7197`) and unsigned device build passes (`phase-4-ui/9cd5abe5d3fa4c19a4cd2f706c45aabb`). Search UI focus/keyboard acceptance remains to be exercised.

## Command admission and navigation

Navigation now advances only when the current-generation command is admitted locally; missing sessions, invalid arguments, cancellation and stale-generation replies return false. Tests cover unavailable, invalid and successful command admission. The simulator suite passes (`phase-4-ui/d9f1da8910ab4912b0596a11bea6707f`). Admission is not network completion: asynchronous Core errors remain visible in the destination state and still require fuller retry/empty-state UI coverage.

### Startup failure recovery

A native session that reported `stopped` previously remained in the home screen's startup spinner. The model now terminates polling with a fixed nonsecret error and retires the service; startup failure displays Retry Stremio. Retry resets navigation and uses the existing serialized session replacement. A real native-store test reproduces the missing error (`5f7c0fa2db45472db1e5ed234e923971`), then verifies error reporting and successful restart after creating the required directory. The full Swift suite passes (`ac3e82d136574cbeb25313ba477d4712`), including the merged Swift account-restoration tests. The retry button itself has not yet been exercised by an automated UI test.

### Restore protected account selection at home startup

The default home model now uses the selected-account service factory, choosing guest only when no protected selection exists. Its label reflects authenticated state. Missing account storage reports startup failure rather than falling back to guest. Unsigned Apple TV build passes (`6a37cd4764ff4090be8d7af2f0eb7a6a`); the factory and cancellation regressions pass in the phase-3 Swift suite. Successful account UI restoration and user-facing login/sign-out remain to be exercised and completed.

### Session replacement for account transitions

StandaloneModel can retire its current service and adopt another only after acknowledged shutdown. A retirement task prevents overlapping run calls from starting while that transition is pending. Generation checks prevent old snapshots/actions from being published after replacement. The caller retains responsibility for an incoming service until adoption succeeds, including cancellation or retirement failure.

The real native-service test verifies old-owner shutdown, cleared old snapshot, a distinct replacement session, and rejection of a delayed old action's error. Full Swift suite passes (`ada90cb3971f468b8285ec733e2bc700`); missing-method reproduction retained (`2b75967136804f1dbacc722676dd2333`). This prepares the ownership transition; the sign-in form and success/cancellation account UI flows remain unfinished.

### Sign-in form and cancellation

Guest users can open an email/password form. Opening joins the prior Core owner; submission owns a fresh account candidate until Home accepts it. Cancellation joins the login task and removes unadopted account storage. Cleanup failure retains the owner for retry. Password input is cleared on submission and dismissal; displayed errors contain no server details or credentials.

The initial combined simulator run (`de72a2cea54e45f38e64b3b4baecdf48`) passed all 45 unit tests and 11 existing UI tests, but the new sign-in UI test failed because upward remote navigation selected the top-right debug Playback Lab button. Giving the sign-in row a full-width focus section fixed that navigation. The unchanged targeted UI test passes (`f19ccbf570f346e9be109f4d3def58a4`), verifying form entry, disabled empty submission, and Menu cancellation back to guest catalogs. An intermediate invocation failed before testing because Cargo was absent from PATH (`c8f71ad0fae440a78fec8f1221a5a007`); the passing invocation used the pinned local toolchain.

A native-backed unit test covers failed submission and clean cancellation without replacing the guest. Successful account authentication through the UI, sign-out, cleanup-failure injection and physical-device behavior remain unverified or unfinished. This slice does not complete phase four.

### Local sign-out and cleanup retry

Home exposes sign-out for authenticated sessions. It invalidates the current model generation, joins the polling task and native owner, clears the matching account selection, and deletes that owner's dedicated encrypted storage and key. Other account directories are preserved. A failed cleanup retains the old owner and blocks run/replacement until Retry sign-out succeeds. The UI clears its entered search and manifest values before starting guest mode. The control is on Home, outside the full-screen playback view; stopping playback as part of a global sign-out action is not yet implemented.

Native-backed tests cover successful cleanup and a rejected mismatched-directory removal followed by retry, including replacement refusal while cleanup is pending. Missing-operation reproduction: `22c7587b7f45497394df08bcf00ad2de`; full Swift suite passes `972ae42383e948588626580553e27b34`. Unsigned device build including the UI passes `dfbdf4b15cfc4334bac542ddc43bbc31`. This does not prove authenticated sign-out UI navigation, deletion failures across process relaunch, or recovery from a selected account whose storage cannot be restored; those remain outstanding.

### Recovery when saved account storage is missing

Home retains the selected account identity before attempting restoration, so startup failure still exposes sign-out. If restoration failed before any native owner was created, explicit sign-out removes that unopened namespace and its key after joining the failed startup task. Existing owners continue through acknowledged native retirement. Startup itself never falls back to guest or creates replacement account state.

The missing-storage regression verifies the visible recovery capability, retained selection before explicit sign-out, cleared selection afterward, and absence of any newly created account directory. Full Swift suite passes (`d6003546ccb948769cc185815d0cfd5b`); missing initialization seam reproduction is `70d1376e2f30432a9f283e8d9c8c2404`. Malformed selection-record recovery and process-crash recovery during cleanup remain outstanding.

### Authenticated restoration through Swift and native Core

A test-only fixture uses CryptoKit to prepare an authenticated encrypted store containing synthetic credentials and the pinned Core schema. It derives default settings from a normally persisted guest profile, then supplies an explicit synthetic account profile. Production code has no fixture-login hook. Two independent Home models restore the account through the normal Swift/C/Rust path, preserve its language, report authentication and sign-out capability, and receive distinct native session IDs. The second model signs out and removes selection/storage. Full Swift suite passes (`4f298369263f4fa1a35f9b21cb75e3a9`). This covers service recreation, not an OS process relaunch, live authentication, token validity or sign-out remote-focus behavior.

### Bind row actions to the displayed revision

All indexed Home actions now carry the revision used to render their row, including catalog/title/episode/source selection and paging. The model refuses dispatch if its current revision differs; the native bridge still performs its own atomic revision validation. Previously the UI captured an index but borrowed a newer revision at dispatch, allowing that index to describe a different row after an asynchronous update.

The native-backed regression advances Core after capturing a displayed revision and verifies that the old row cannot dispatch. Full Swift suite passes (`a50d127f4f3244048e22e32c3d239c0d`). Removing the admission guard reproduced both unexpected dispatch and acceptance (`2a3fe495cbe740bfa8ade69131d7bbf1`); the verified guard was restored byte-for-byte afterward. This protects selection identity but does not yet add season filtering or focus restoration.

### Season-filtered episode selection

The metadata snapshot now includes selected season and sorted season choices (bounded to 128 choices; All episodes preserves access to the complete paged video list). Selecting a season is revision-checked, rejects absent seasons, and resets the video offset. Pagination and source selection both use the same filtered video list. Changing title or signing out resets the filter. The UI offers All episodes, numbered seasons, Specials for season zero, and season/episode labels on video buttons.

An interleaved two-season fixture with identical episode titles verifies filtered paging and selection of the exact final video ID. The simulator HTTP fixture exercises Swift → C → native season selection, filtered counts and reset to all episodes. Full pinned Rust 1.100.0 Core suite passes (`336811b97f7145eb91b785953aac795e`), and full Swift suite passes (`4d621e3401a145a5a31f4b74f64808e0`). Initial missing-filter reproduction: `2cf5cbaed60d4a44b7250334dbff2c90`. Remote-focus behavior for the new picker, UI screenshots and full source-to-playback journey remain outstanding.

### Combined account/season validation

At `ce014f6`, the combined simulator run passes all 50 Swift unit/integration tests and 12 short UI tests (`dcb12ed743a54e71a7f436bfff7fefdf`); UI execution was 116 seconds. Updated unsigned device compilation passes (`418fb02ee57541eb92ed60d6fac21027`). Screenshot review caught the guest view showing through the sign-in cover. An opaque full-screen background fixes it; the unchanged targeted form/cancel UI test passes (`828216d3626b46fe9445936141479879`) and its exported screenshot was visually checked. No physical-device acceptance is implied.

The prior GitHub phase-four run `44135013503` failed to acquire macOS ARM runners; its native job was cancelled with no executed steps and a runner-capacity annotation. This is separate from code/test failures and cache performance.

### Direct HTTPS admission boundary (integration pending)

Added a typed private HTTPS handoff payload and validation for direct HTTPS URLs. Signed query parameters are preserved. Non-HTTPS schemes, URL user-info, fragments, port zero, URLs over 16 KiB, external-player sources and nonempty required proxy headers are rejected. VLC cannot honor an add-on proxy contract through a plain direct URL, so those sources must remain visibly unsupported rather than silently dropping headers. Payloads have no Debug implementation and are not presentation snapshots.

Pinned native source-validation tests pass (`5cd156ffbc154a9796050bdc8ae6b1a1`), after missing-entry-point reproduction (`94b107c9d7934e18829ff8ac1675b31a`). This is only the admission layer: C/Swift handoff, decoder ownership, transport/redirect behavior and HTTPS playback fixtures remain to be implemented and verified. HTTPS source buttons are still disabled.

### Typed HTTPS Core handoff

Core now prepares HTTPS source data with title/video identity under the same visible-source-window and revision checks as torrents. `cue_core_request_https` shares the existing one-ticket slot and take/discard operations: torrent/HTTPS requests cannot overwrite each other, and stop invalidates the request. Swift decodes a typed HTTPS payload through the shared bounded, timed handoff reader. The signed URL is returned only to the requesting caller, not added to snapshots.

The full pinned native suite passes (`031d1d023c6a41108b2320906c6dfc76`), including mutual exclusion, unavailable-source errors, consumption and stop rejection. Missing-C-entry reproduction: `a0f1d3544b3f453695162b72c616d49b`. The full Swift suite passes (`fdf8a44f87f4419385bd24215e6403bb`); its HTTP add-on fixture supplies a synthetic HTTPS source whose signed URL and exact title/video identity are asserted, plus stale-revision and wrong-source-kind rejection. No external media request occurs in this fixture. Decoder ownership, HTTPS transport fixtures and UI activation remain pending.

### Shared torrent/HTTPS playback lifecycle and TLS proof

The production source view/model now accept typed torrent or HTTPS requests. Direct HTTPS uses the same VLC surface, coordinator deadlines, pause/seek/track controls, generation checks and decoder retirement; it allocates no torrent cache. Source buttons dispatch the matching private Core handoff only on selection. URLs requiring proxy headers and incompatible URL schemes remain unsupported. Lifecycle tests verify decoder stop on replacement/exit, stale callback rejection, no torrent cache, and rejection of unsafe URLs before decoder admission. The full Swift suite passes (`19fd77633af44d778d3fc592dd3546c3`); pinned native Core suite passes (`3939be5556c649ad8bd646447d33f086`); unsigned device build passes (`ec579a9457bd40a2b91c5bbc1a2fe924`).

Run the separate opt-in TLS check with the configured pinned toolchain on PATH:

```sh
python3 scripts/agent/task.py verify phase-4-ui bash scripts/fixtures/check-https.sh
```

It generates short-lived test certificates, serves only the generated episode fixture on loopback ports 42425/42426 with byte ranges, creates a disposable tvOS 26.5 simulator, trusts the test root only there, and runs `HTTPSPlaybackTests`. The trusted endpoint must advance video and close back to Home. The untrusted endpoint must report failure without any observed playback advancement. A DEBUG-only fixed-loopback fixture entry uses the production SourcePlaybackView; release builds exclude that entry. No TLS verification bypass is installed in the player. The script always removes its simulator/certificates and stops its servers on exit.

Initial TLS pair passes (`4959761fca104b2bae1aa3e04af17b8d`); the strengthened no-advancement rejection test passes (`eaa520c74f9e4b2c81be16247f60f8ed`). The full repeatable script passes (`cd66b71b8c0a4ba7949dba324cbae313`), and cleanup was independently checked. This opt-in proof is not added to automatic short CI. It proves local trusted/untrusted TLS behavior and initial playback, not arbitrary redirects, HTTPS seeking under interruption, a live add-on journey or physical-device playback.

### HTTPS controls and player focus

The trusted TLS UI test now verifies pause stability, backward seek to zero, resumed advancement and remote navigation to Close. It reproduced a focus gap from the controls to the centered Close button (`931a7873516d43b1bbaa7b7c26b5b1b7`). A full-width focus section around Close fixes it; both TLS tests pass (`065da1cb51f74de9bef72f139ef56882`). Screenshot review then exposed Home content behind the transparent player cover. An opaque background fixes that presentation; the unchanged trusted playback/control test passes (`4e8be2b6959e4260bbfd27f5595a948c`) and its screenshot was visually inspected. The fixture runner accepts an optional XCTest selector for targeted reruns. These are small local-file TLS fixtures, not proof of uncached large-file seeking or interrupted remote transport.

### Remote Back and selection focus

Standalone Home now handles remote Back through sources → details → titles → catalogs, retaining the row the user selected. Title/episode focus uses content identity; the root leaves Back to tvOS. Playback dismissal requests focus on the selected source. The source-dismissal and movie paths still need journey coverage.

An opt-in owned add-on fixture installs through the actual native Core into an ephemeral guest service and uses the production Home. It supplies three series and two seasons of episodes over loopback HTTP; it neither fakes UI snapshots nor uses account storage. `StandaloneJourneyTests` navigates with the remote to the second title and season-two episode, then verifies episode/title/catalog focus on Back. The first run reproduced missing remote Back (`791e67532a144e3c9d5ff12c89f7c718`); a subsequent run reproduced title focus being overwritten by transient focus (`976906f14db44a1a89b5fe2ef6001275`). Recording focus when a row is selected fixes that transition. The complete script passes (`6170e5578811466d81107a08930d126e`), including all three return levels and server cleanup.

With the configured toolchain on PATH, run:

```sh
python3 scripts/agent/task.py verify phase-4-ui bash scripts/fixtures/check-journey.sh 'platform=tvOS Simulator,id=YOUR-SIMULATOR-ID'
```

This remains outside automatic short CI. Its torrent source is synthetic; this navigation test does not select it or prove transfer/playback. The full correct-episode torrent playback journey, add-on subtitles, artwork, live add-on and physical-device gates remain outstanding.

Regression verification for the navigation change: `make apple-ci-test` passes 52 Swift tests and all 12 short UI tests (`1fde07615b23439683190dad8e018ce0`, UI execution 114.45 seconds); `make apple-build` passes unsigned device compilation (`bf4069233ab8409e941a68318d2a5ef1`). The restored-catalog screenshot was inspected. Portable harness checks pass all 15 tests. No physical-device claim follows from these checks.

### Complete owned catalog-to-peer playback journey

The opt-in journey runner now starts an isolated multi-episode seeder on loopback port 42428 and an HTTP add-on/tracker on 42427. The tracker returns only the owned peer; the HTTP server provides no video route or web seed. The seeder uses the existing pinned torrent engine and copies only the generated fixture files into a temporary directory. Its tracker/DHT/local-service discovery are disabled. The production app uses its normal Core source handoff, tracker discovery, torrent cache/byte adapter and VLC player; no playback-specific app shortcut was added.

`testSelectedEpisodePlaysFromPeerAndCloses` navigates to series 2 / season 2 / episode 2, asserts the stream response identifies `cue-owned-2:2:2`, starts playback and waits for the clock to advance. It independently inspects the decoded frame: episode two is green, while episode one is blue and the tail-index file is red. It then closes playback, waits for dismissal/focus to settle and verifies source → episode → title → catalog focus. Both navigation and playback tests pass on the final content (`b5171f68c704405281dfa7925ca25c29`). The fixture services are no longer listening after exit, and the script removes its owned temporary directory.

A wrong-file mutation (`fileIdx: 0`) preserved advancing playback but failed the frame assertions with blue 255 / green 15 (`82a0607b816c46efa50c7081883a2aeb`). The correct fixture was restored byte-for-byte before the final passing run. Initial immediate-after-dismissal focus assertions failed (`c5a54d4544764d1595680fab29bff7ea`, `4bdc48b3fb5443ea9f477cb4ab278a1f`); the test was observing the transition before focus settled. The proposed extra Home focus-reset change was removed, and the final test passes with the previously shipped app code. Thus this follow-up adds verification infrastructure, not an unproven production fix.

The same `check-journey.sh` command now runs both tests; an optional second argument selects one XCTest method. It remains opt-in rather than extending push CI. Portable harness checks pass all 15 tests; shell/Python syntax and diff checks pass. The full real-peer simulator journey is now covered, but search UI, season-picker interaction, movie paths, live add-on playback, subtitles, artwork and physical-device qualification remain separate outstanding gates.

### Stream-provided subtitle tracks

Selected torrent and HTTPS handoffs now carry up to 32 validated subtitle descriptors from the Core stream response. Configured URLs remain private handoff data, not presentation snapshots. HTTP(S) URLs are bounded; user-info, fragments, local-file URLs, oversized labels/IDs and font-dependent tracks are omitted. Existing handoff payloads without the additive subtitle field still decode.

The playback owner downloads a chosen track into an independently named temporary file. It accepts UTF-8 WebVTT/basic SRT with timing cues, caps each decoded body at 1 MiB and the per-owner cache at 32 files, permits at most three validated redirects, and refuses HTTPS-to-HTTP redirects. Its ephemeral URLSession has no cookie, credential or response cache. Request/resource timeouts are 15/20 seconds. Switching cancels and joins previous work; Off cancels a pending download; Close joins retirement; decoder shutdown precedes temporary-file cleanup. A selection ID prevents an obsolete decoder error, including one for the same URL, from clearing a newer choice.

The player exposes stream subtitle choices and Off through a remote-reachable focus section. VLC tracks are reused rather than appended again on every toggle. Re-enabling a previously loaded long cue required a current-position refresh on seekable playback; the fixture verifies the caption returns without restarting from zero. Non-seekable subtitle re-selection remains unqualified.

Missing-module/downloader reproductions are retained (`b24bb8944db84ea28c42c6331d518c38`, `2f33b3b557454c9c85843bf65ecb0f22`). The full pinned native Core suite passes (`ffdd56aabf63457686275c5d07702909`). Local network-boundary tests cover valid download/cache cleanup, unsafe URLs/redirects, declared and streamed size limits, non-subtitle bodies, caller cancellation, stalled-transfer shutdown, Off during loading and late-error rejection. The current 57-test Swift suite passes (`e752a7e0e62843cd8032f10806c98dc8`). The short regression run also passed 57 Swift tests and all 12 existing UI tests (`33bbc6b60e264e7288ae2a06267f4091`) before the final selection-ID refinement.

The opt-in real-peer UI test checks actual caption pixels, not just a track label. It reproduced an unreachable subtitle row (`fd72889c8acf466993ccaa2fd29bd06a`) and failure to render a re-selected long cue (`f230f92b803e4b1ab967350288fa233f`, `f3511aeda3974ed5a7d2022078b85f85`). The focused rendering/Off/re-selection/remote-Back proof passed after the fixes (`7a1e3f46439645a4b348534bb1c81ed7`); the rendered screenshot was inspected. The test remains outside automatic push CI.

This covers subtitles supplied inline with a selected stream. Separate add-on `subtitles` resource discovery is still outstanding: pinned Core's Player subtitle updater currently depends on a converted streaming-server source. The standalone path must use Core's add-on resource transport without inventing a server or weakening the direct-torrent requirement. Artwork, remaining browsing cases, live add-on/account checks, recovery and physical-device qualification also remain open.

Final verification after selection-ID refinement: all three opt-in journey tests pass (`cf42ba4161f144cab8b7c5659ccab02b`, 78.91 seconds), including rendered on/off/on captions, preserved playback position and remote Back. Unsigned device compilation passes (`9e0025377cd947b89ea8d776e1317644`); all 15 portable harness checks pass. Later Core progress integration must treat the subtitle current-position refresh as a seek.


### Installed add-on subtitle discovery

Playback now discovers separate subtitle resources after the video URL is ready.
Core's manifest planner filters providers by resource, video type and ID prefix.
The selected title/video/type and source generation are checked by the owner
before fetching and again before returning private results. No fabricated file
hash or size hints are sent. Discovery uses Core's real HTTP transport, at most
32 planned providers, four concurrent requests, three seconds per provider and
four seconds overall. Successful partial results survive provider errors and
are deduplicated and capped at 32 tracks.

The versioned C request uses the existing bounded private reply slot; every slot
owns a cancellation token canceled by take/discard/retirement. Swift shares the
256-KiB/five-second reader with source preparation. Playback captures the Core
owner/generation, independently starts discovery when ready, merges inline and
discovered tracks, and shows provider errors separately from playback errors.
Close cancels and joins discovery; late results cannot reopen the closed model.
Retry identity includes playback generation even when its URL is unchanged.
No new external dependencies or automatic extended tests were added.

The owned fixture serves captions only from the selected episode's separate
resource (`cue-owned-2:2:2`), with no inline fallback. The opt-in journey verifies
peer playback, rendered caption pixels, off/on re-selection, playback-position
preservation and remote-back focus restoration. Video bounds are explicit for
dynamic controls. The pixel helper now uses a screen screenshot and its image
scale: diagnostics proved XCTest sometimes returned a zero app frame and 1×1
app screenshot despite a valid video surface. Pixel thresholds remain intact.

Evidence (task `phase-4-ui`):

- Missing planner red `7d7b9cdcc21a43e7ae05e38491093945`; compatible-provider
  planning green `c6c9ba87997f413a80707fe57812513b`.
- Real HTTP failure isolation/deduplication/socket-close cancellation
  `ae4476ce1dbe4bc7af7116dbf40c2302`; fanout and total-deadline accounting
  `ee2a533b1a9f45d1b1381f9f31f6f35f`.
- Session/identity red `1f31485e2d9d47deae3a3cd2adf305f0`, green
  `27632e65a87f4c4cbd05b94494f0e4c4`; C boundary red
  `4c19234532f54993bc1e36c2e9bc8d8c`, green `04984566ae9a483bb292ef899dfedfa6`.
- Missing UI choice red `d3b0138412e44614bb59bb1302256539`; rendered-caption
  capture failures `2129bf7fc46d44c0b07b1830fe96091e` and
  `dbb67f0cce414a0ab42ccf29fdaa77f0`; actual invalid screenshot/zero app frame
  diagnostic `f0e16a70cd404b0eadb4185abc8fb101`; separate-resource journey green
  `aa174083e6c04a9fb1ea40400d5dc5f4` (only evidence docs edited during run).
- `4e146e09ae304384b4237e418f9a0258` selected zero tests due to a mistaken
  target-prefixed selector and is explicitly NOT evidence of a passing journey.
- Full pinned Core suite `647cfefad7454c25abc213275539458c`; final 59 Swift tests
  `5a6ae060aa9249b3a1e656c7ddb79161`; unsigned device build
  `6c48fb5b8d2344dca95e41abff66e1fc`.
- Swift coverage includes real separate-resource success, cancellation closing a
  stalled provider socket before its timeout, prompt native command-loop reuse,
  close rejecting a late result, serialized replacement and partial success.
  The first extended fixture run `d3d1ffc2fa0b4401b9551da96733df17` caught its
  stale test revision after a language update; the test now refreshes that state.
- Combined short regression `7f23e17225cc4dbba3a8a29883ad5aae` passes 59 Swift
  tests and 12 existing playback UI tests (only evidence/status docs changed during
  the run). `make check` passes all 15 portable harness tests.

Limits: hash-dependent subtitle providers are not supported without known media
hints. Live permitted add-on and physical-device validation remain open. Phase 4 and the overall implementation stack remain draft.


#### Renderer resize and manual fixture follow-up

Replay `354c69a3325d4ae1b8107db644a6077b` showed no caption despite a selected
track. An attempted first-load position refresh did not resolve this reliably
(`b8e22fe322c646e5b19576c71d88e7d0`) and was removed. The 12-second media also
ended during remote assertions, so the manual runner now generates a separate
approximately 36-second multi-file torrent by stream-copying the existing owned
colors/audio three times. It checks actual duration, records file indices,
checksums and generator version, and supplies the actual hash to the fixture
add-on. The short automatic CI media remain unchanged.

The longer-fixture failure `20c68a2a00a84848b6e59dd30358a38b` ruled out duration
alone. VLC's attached rendering view now follows its container's bounds when
asynchronous controls alter available space, keeping the caption area visible.
Targeted verification `7c153156f5a5462486514afbadb0fd93` passes rendered on/off/on
captions, position and remote navigation. The existing re-selection position
refresh remains and must be represented as a seek in phase-five progress
integration; nonseekable re-selection is still unqualified.

Final combined regression `bfd7749ce1184870afa3790fccd8bf85` passes all 59 Swift
and 12 existing UI tests after the renderer fix. Only manual-fixture metadata
logging changed during that run, not application or test code. Python fixture
scripts compile and portable checks pass. Final unsigned device build
`05b9a8e5cce64336a300dd9c17c80cb7` passes.


In-flight selection proof: the real HTTP fixture holds a subtitle response while
Core reselects the same title/type/video. The response is rejected solely because
its generation is obsolete (`9162d60dda834ae8adb3080b0893ed22`). Temporarily
removing that generation comparison makes the new assertion fail
(`ba27db07e3b845dfb0079d4dd01d1ef5`). The exact production source was restored;
all six subtitle tests then pass (`d6e249583e6b4464a5faebd87a41f426`). This closes
the earlier in-flight selection-change coverage gap without changing behavior.

### Source provider grouping

Source snapshots now carry a bounded public add-on name and installed-provider
index, resolved from the private resource base URL. Configured URLs remain out
of the presentation data. Embedded metadata streams and separate stream pages
retain their original flat selection indexes and paging semantics. Swift groups
by provider index, not name or manifest ID, preserving distinct configurations
and exposing an Unknown add-on fallback when identity is unavailable.

Native tests use two private configurations of the same manifest ID, checking
labels, cross-provider paging and URL exclusion. Missing-API red:
`a014850a64ab4add8250fbdf269dbd8f`; native details/catalogue green:
`5cc830805a2149b78fe4a8f44940fc6a`. Swift grouping red:
`5800817d2b2f4f7ea744c603dc384f20`; all 60 Swift tests pass:
`3dadf6fbd84f4941b74324ce17c94737`. The unit fixture proves that equal display
names do not merge providers or renumber selections. The real UI/peer journey
asserts the add-on heading, correct green episode, Close and focus restoration
(`a08a38f7d1154369a4da58a4cb6a36b1`). Full Core suite:
`2c4ca89d4efa4706be08d839aa6c1a73`.
Unsigned device build `08d8250e1ee44e83a7a7f5dd9d251548` passes (only this
evidence document changed during the run); all 15 portable tests pass.
