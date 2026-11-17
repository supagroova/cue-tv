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
