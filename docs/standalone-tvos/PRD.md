# Cue TV — standalone Apple TV player

Version 0.1 · 17 November 2026 · Draft for product review

## 1. Product decision

Build a personally sideloaded Apple TV app that discovers Stremio content and streams raw torrents directly inside Cue TV. Browsing, source selection, torrent transfer, decoding and playback must all run on the Apple TV. Normal use must not require Android TV, DietPi, a Mac, Transmission, a paid debrid account or a separately installed player.

Internet services remain necessary for accounts, add-ons, metadata and peers. “Standalone” means no companion computer or home server, not offline operation. A Mac remains necessary for development and signing; optional phone-assisted account/add-on setup does not make the phone a playback dependency.

The user explicitly confirmed direct torrents without paid debrid and a personal sideloaded app limited to Stremio content. Everything below marked **proposed** is a product recommendation for review, not an additional confirmed preference. This PRD defines outcomes and release gates; it is not the detailed implementation plan.

## 2. Problem and intended user

The user has an Apple TV and a DietPi Raspberry Pi, but no Android TV. Cue TV’s current Apple TV target compiles and launches, yet depends on an Android service for its library, source resolution and pairing. It opens another application for playback. That prevents the desired one-app viewing experience on the existing hardware.

Primary job: “From my sofa, find a film or episode, choose a source, start watching before the whole torrent downloads, and resume later without managing servers.”

Success is a real-device session from launch to playback with the Pi and Mac unavailable, followed by successful resume after relaunch. It is not merely a successful compilation, simulator screen or playable HTTPS/debrid source.

## 3. Baseline and evidence

The inspected Cue TV snapshot has a SwiftUI interface, validated Stremio deep links, a local pairing client and an Android-backed service abstraction. Its current source-selection implementation filters for Torrentio/Real-Debrid HTTPS sources. There is no embedded torrent engine or in-app media player in the inspected Apple TV target. [Cue TV source](https://github.com/rclaycock/cue-tv/tree/main/apple-tv/Sources), [source selection](https://github.com/rclaycock/cue-tv/blob/main/on-tv/android/app/src/main/java/dev/couchtv/ontv/StremioPlayback.kt)

Local baseline evidence from 16 November: device-target compilation succeeded with signing disabled; five existing unit tests passed after simulator startup was completed; the app displayed its connection screen. Those tests do not establish media playback or networking capability. See [build evidence](LOCAL-BUILD.md).

The [research memo](RESEARCH.md) distinguishes inspected code, upstream claims and untested proposals. No torrent engine or replacement player was integrated during this investigation.

## 4. Scope

| Required for v1 | Explicitly outside v1 |
|---|---|
| One local user profile and optional Stremio account connection | Multiple household profiles and parental controls |
| Installed add-ons, catalogues, search and metadata | YouTube, trailers, live TV/IPTV and other providers |
| Films and series with season/episode selection | A custom iPhone remote or iPhone app |
| Direct torrent source selection and streaming | Required debrid subscriptions or external torrent servers |
| In-app playback, seeking, audio/subtitle selection and resume | External-player handoff as the primary playback path |
| Library and watched state; account synchronization when connected | Android control, HDMI routing, replacement system launcher |
| Bounded temporary media cache and useful failure messages | Permanent download library, background download queue, ratio-management client |
| Personal signing, installation, upgrade and removal instructions | App Store release, public distribution and automatic app updates |

Compatible direct HTTPS sources may work through the same player, but they cannot substitute for the direct-torrent acceptance gate. Proposed v1 torrent scope is public BitTorrent v1 metadata/magnets, including multi-file torrents. Private-tracker credential workflows and v2-only torrents are deferred; unsupported sources must be identified without silently failing.

Confirmed target device (user report, 17 November 2026): **Apple TV 4K A2737, 64 GB, tvOS 27.2**. Apple identifies A2737 as the third-generation Wi-Fi model; acceptance testing must therefore use Wi-Fi. [Apple model identification](https://support.apple.com/en-us/101605)

Proposed first release playback baseline remains 1080p SDR. 4K/HDR and enhanced audio are stretch capabilities until validated on this device. The earlier tvOS 17 minimum is a provisional deployment setting, not a commitment to test older systems. Verify Xcode/device support for the reported tvOS 27.2 during pairing; previous simulator/build evidence does not establish compatibility with this OS. The 64 GB capacity is total device storage, not an available cache budget.

## 5. User journeys

### First run

Launch Cue TV without an Android address or pairing code. Explain account connection and offer a guest setup using a known working catalogue/add-on configuration. Connect a Stremio account using a supported upstream flow when desired; keep credentials out of logs. Show installed add-ons and whether each is reachable. A phone browser may assist with login or copying a configured manifest URL, but is optional during viewing. Preserve normal Siri Remote and system text-entry behavior.

### Browse and play

Browse a catalogue or search by title. Open details; for a series choose season and episode. Display sources grouped by add-on with whatever quality, language, size and transport information is actually supplied. Missing size or seed counts are “unknown,” not zero. Selecting Play starts one source, with visible progress through metadata acquisition, peer discovery, buffering and playback. No peer session starts simply by focusing a poster or viewing sources.

### Control and resume

Use the Siri Remote to pause, seek, choose audio/subtitle tracks and exit. Seeking into an undownloaded region reprioritizes the necessary pieces. Returning to the source list stops the prior session before a new source starts. Returning to the app offers Resume for the same episode and position. Playback position is associated with the title/video identity rather than one transient source URL.

### Recover from failure

When a source has no peers, a tracker fails, storage fills, the network drops or a codec is unsupported, explain the failure and provide Retry, Choose another source and Back as appropriate. Cancellation remains responsive while requests are pending. Do not display an endless spinner or automatically switch to a paid service.

## 6. Functional requirements

| ID | Requirement | Acceptance evidence |
|---|---|---|
| FR-01 | All runtime functions execute in Cue TV on Apple TV; remove Android pairing from standalone mode. | Complete browsing and direct playback with Pi off, no Android host, Mac disconnected and no other player installed. |
| FR-02 | Integrate Stremio Core behind a versioned Swift boundary for account, add-on and library state. | Auth success/failure, guest setup, state changes and sign-out tested; token material never appears in diagnostics. |
| FR-03 | Load add-on manifests and supported catalogue, metadata, search, stream and subtitle resources. | Fixture contract tests plus a permitted live add-on test; one failing add-on does not prevent others returning results. |
| FR-04 | Support film and series details, explicit episode choice and user-selected playback source. | Correct video/file is selected in a multi-episode test torrent; selection never relies only on the largest file. |
| FR-05 | Handle torrent sources described by info hash/file index and supported magnet metadata; validate source data before use. | Pure torrent fixtures start without any debrid credentials or pre-resolved HTTPS video URL. |
| FR-06 | Embed peer discovery and transfer; prioritize initial playback and seek targets; verify downloaded pieces. | Test HTTP(S) trackers, UDP trackers and DHT separately on the physical device. Required protocols must work or trigger a recorded scope decision before v1. |
| FR-07 | Play inside Cue TV with pause, seek, track selection and subtitle on/off. | Baseline media matrix passes; unsupported variants produce an actionable message instead of a crash. |
| FR-08 | Keep only one user-selected torrent playback session active. Cancel obsolete metadata, buffering and seek work. | Repeated source changes produce neither stale playback nor orphaned transfers. |
| FR-09 | Save progress at bounded intervals and on pause/exit; reconcile account state when connected. | Relaunch resumes within 15 seconds of the last saved position; retry queued sync after a network outage. |
| FR-10 | Allow library add/remove and watched-state changes. | Changes survive normal relaunch and reach the connected account; conflicts follow a documented rule. |
| FR-11 | Enforce a media-cache budget and free-space reserve. Cache loss must never corrupt account state. | Low-storage and cache-purge tests pass; deleting cache cannot delete credentials or watched state. |
| FR-12 | Handle backgrounding as suspension, not an always-on service. Save state and stop active transfer when leaving viewing mode. | Home, sleep, interruption and return do not lose the episode selection or leave uncontrolled sessions. |
| FR-13 | Provide source errors and a compact diagnostic view: engine state, peers, rate, buffer, cache use and error category. | Diagnostics distinguish missing peers from decoder failure; sensitive URLs, keys and tracker passkeys are redacted. |
| FR-14 | Install and upgrade under the user’s own bundle ID/team without changing the brother’s install or other apps. | Signed device install, same-identity upgrade, launch and removal instructions verified on the target device. |
| FR-15 | Preserve tvOS focus, Back behavior, readable text and accessible labels. | Every primary flow is usable with the Siri Remote; no focus trap during loading, failure or player exit. |

Sign-out must stop playback and clear local account secrets, configured account-specific URLs and pending account writes. Guest preferences and account data must not accidentally merge between identities. Add-on configuration URLs can contain credentials and must receive the same protection as tokens.

## 7. Playback and resource contract

Proposed release matrix: MP4 and MKV containers with 1080p H.264 SDR and AAC stereo, plus a tested HEVC SDR profile on supported hardware; selectable embedded audio and basic SRT/WebVTT subtitles. List exact profiles and reference files before development estimates. ASS styling, image subtitles, Dolby Vision, HDR10, Atmos/TrueHD/DTS passthrough and AV1 are not implied by “plays torrents.” Each additional format requires evidence and a deliberate scope decision.

The engine must serve byte-range reads without waiting for a complete download. Header/index reads, cancellations, end-of-file handling and seeks beyond the current buffer must be supported. A file larger than the configured cache must either work through verified bounded storage or be rejected explicitly before uncontrolled growth. Rolling eviction/re-download is an unresolved engineering choice, not a capability assumed of a candidate library.

Proposed transfer policy: ordinary peer uploading may occur during the active session; stop the session on playback exit or app backgrounding. No unattended seeding promise or tracker-ratio management. Expose this behavior clearly in settings. Do not auto-add trackers to private torrents or bypass their discovery restrictions.

## 8. Quality targets and acceptance methodology

These are proposed release targets, not measured performance claims. Run them on the confirmed physical Apple TV with owned/permitted reference videos. A controlled test seeder may be used to measure the client; it is a test fixture, not a production home-server dependency. Also perform a permitted public-swarm smoke test, whose latency is recorded separately.

| Test | Proposed passing result |
|---|---|
| Source selection → first frame | At least 9 of 10 cold-cache starts within 20 seconds on a controlled healthy swarm; record metadata, discovery and buffer timings separately. |
| Seek | At least 9 of 10 seeks into uncached regions resume within 10 seconds on that swarm. |
| Sustained playback | Three 60-minute baseline-media sessions with no crash, incorrect episode or sustained audio/video drift. |
| Controls | Pause, Back and cancellation respond within 1 second even while peer discovery is stalled. |
| Recovery | Network loss, zero peers, invalid metadata, unsupported codec and disk-full tests produce finite failures and working retry/back controls. |
| Session cleanup | Twenty source-switch cycles leave no abandoned transfers, steadily growing cache or unbounded memory growth. |
| Resume | After ten pause/exit/relaunch cycles, return to the correct episode within 15 seconds of saved position. |
| Independence | Repeat the complete user journey with the Pi off and the development Mac disconnected. |

Measure memory, CPU, thermal behavior and actual disk use during sustained tests. Set numeric memory and cache ceilings after measuring the target device and completing the engine spike; passing means staying within those ceilings without OS termination. Do not choose an arbitrary cross-device memory allowance now.

## 9. Technical direction — proposed, not locked implementation

Retain Cue TV’s native tvOS interface and adapt its service boundary. Embed a pinned Stremio Core revision using a narrow C ABI and a platform environment for networking, storage, time and task execution. Do not rewrite Stremio’s full account/add-on model in Swift or introduce a broad Android emulation layer.

Use a separate torrent session abstraction to select files, obtain media bytes and expose status. Evaluate **librqbit** first for a shared Rust toolchain and existing seekable streaming support, with **libtorrent** as the alternative. Neither has passed our tvOS device test. Choosing a library requires build, networking, cache and playback results, not just a successful simulator compile. [rqbit streaming](https://github.com/ikatson/rqbit#streaming-support), [libtorrent streaming](https://www.libtorrent.org/streaming.html)

Keep the media engine replaceable. Evaluate VLCKit and MPVKit against the agreed media matrix; Apple’s player can validate a narrow first MP4 flow but is not evidence of broad MKV support. Avoid multiple production playback engines in v1 unless the compatibility evidence requires them.

If a local HTTP range adapter is used between the torrent engine and player, bind only to loopback and scope requests to the active session. Do not expose a LAN media/control service. All runtime dependencies must operate in-process without external FFmpeg processes, an external Node server or another installed application. Native libraries may have their own worker threads.

The proposed dependency policy is source-buildable libraries with pinned revisions, preserved notices and a reproducible build recipe. A public binary download is not automatically a reusable open-source component. StremioX is a valuable integration reference, but its bundled server/runtime is not the selected dependency. See the research memo for provenance and limitations.

## 10. Data and operations

Store secrets through platform-protected storage. Keep transient media separate from preferences, library state and queued sync. Make cached state reconstructible when tvOS reclaims storage. Cloud-backed account state must continue to work after cache loss; guest-state durability must be characterized and honestly described during acceptance.

Validate untrusted manifests, torrent paths, file indexes, URLs, headers and resource sizes. Constrain file writes to the media sandbox and bound requests and retries. Do not permit arbitrary add-on responses to select filesystem paths or invoke executables. Default to encrypted remote account/add-on requests; any necessary compatibility exception should be narrow and documented. Diagnostics must not upload viewing data automatically.

Deliver a signed development build for the user’s registered device, an exact source/dependency manifest, build instructions, test results and signing renewal instructions. Do not promise perpetual installation or a signing lifetime until the user’s actual account/profile is verified. No changes to Sonarr, Transmission, SMB or the Pi are required by this project.

## 11. Risks and release gates

| Gate | Evidence needed before proceeding |
|---|---|
| G0 — device and baseline | Recorded A2737 / 64 GB / user-reported tvOS 27.2; verify OS/build during pairing, Xcode device support, signing capability, available storage, media matrix and product scope. |
| G1 — embedded feasibility | Source-built Core bridge and a torrent engine run on device; peer protocols and seekable bytes proven; dependency provenance recorded. Failure blocks the estimate and prompts a design decision. |
| G2 — vertical playback | One permitted raw torrent plays inside Cue TV, seeks and stops cleanly with no paid service or external app. |
| G3 — product integration | Account/add-ons, search, episodes, sources, library and resume work together. |
| G4 — personal release | Quality matrix, interruption/cache tests, independence test, signing/install and handover complete. |

These gates define outcomes for a later implementation plan, not detailed tasks or a calendar. The plan must map each requirement to tasks, dependencies, tests, deliverables and rollback behavior. Architecture choices and estimates should follow G1 evidence. No schedule is committed in this PRD.

Highest risks: unproven tvOS peer networking in candidate engines; cache behavior for files larger than the budget; media compatibility; Rust/native-library integration; upstream API changes; signing and device availability. StremioX code comments about its own UDP failures are not proof that all tvOS UDP or DHT is impossible.

## 12. Decisions for review

Confirmed: direct torrent playback; no required debrid; one Apple TV app; personal sideloading; Stremio-only v1; no Android device available.

Proposed defaults: one profile; guest and account-connected modes; 1080p SDR first; public BitTorrent v1 scope; source-buildable dependencies; no unattended seeding; no public distribution. Review these before making the implementation plan definitive.

Still needed: device pairing and toolchain compatibility verification, desired baseline media formats, signing account capabilities and whether changes should eventually be proposed to the brother’s repository or kept in a separate branch/project. No remote branch, pull request or message to the brother has been created.

The next document, after product review, is the detailed stage-by-stage implementation plan. It must include explicit decision branches for the torrent engine and player rather than assume those risks have already been resolved.
