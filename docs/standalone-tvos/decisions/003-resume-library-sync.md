# 003 — resume, library and account recovery

Status: selected for implementation; no Phase 5 completion claim.

## Inspected behavior

The checked-in Stremio Core is authoritative for these observations:

- `models/player.rs`: `PUSH_TO_LIBRARY_EVERY` is 180 seconds. The helper
  `push_to_library` throttles `Internal::UpdateLibraryItem`. A playback clock
  callback alone therefore does not establish the PRD's 15-second resume bound.
- `ActionPlayer::Seek` updates position/duration without adding watched time.
  `TimeChanged` accumulates positive position deltas; passing a forward seek as
  normal playback can falsely count skipped content as watched. Backward position
  changes need Seek too. Caption reloads that reposition VLC follow the same rule.
- `constants.rs`: automatic watched threshold is strictly greater than 70% of
  duration in accumulated watched time, not 70% playback position.
- `models/ctx/update_library.rs`: add/remove/manual watched actions update Core
  library items. Removal is a tombstone, not erasure of the record. Local storage
  and authenticated remote writes are separate effects; a remote error is surfaced
  as an event. Successful dispatch is not proof either write completed.
- `plan_sync_with_api` compares modification times using whole-second timestamps.
  Equal-second records produce neither a push nor a pull in that planner. The
  item merge path accepts equal or newer timestamps. Neither behavior constitutes
  a conflict policy with a durable retry guarantee for this application.

Paths above are relative to `native/stremio-core/src/`. These findings describe
this pinned source, not all Stremio versions or behavior of other clients.

## Local state and playback contract

Core library state remains the source of title/video progress and watched state.
Do not create a parallel Swift library database. The native session owns a Core
Player model and the selected Core stream identity. UI snapshots omit private
stream URLs; a resume record retains title type, title ID and video ID and resolves
sources again. A signed source URL is never the durable resume key.

Use a 5-second foreground checkpoint cadence, plus explicit checkpoints on pause,
seek completion, Close, scene deactivation and orderly owner replacement. Persist
through the existing identity-scoped encrypted storage. A checkpoint is complete
only after durable storage succeeds; expose a recoverable save error on failure.
Serialize checkpoint writes so a delayed older write cannot overwrite a newer
position. Bound/coalesce pending periodic checkpoints to the latest one, while
retaining explicit lifecycle barriers. The 5-second choice provides margin for the
15-second acceptance tolerance; only measured relaunch tests prove that tolerance.

Implement an explicit Core checkpoint integration instead of relying on the
180-second helper. Retain the upstream interval for unrelated callers unless a
small, documented integration seam requires a vendor change. Checkpoints must not
force a network upload every five seconds. Local durability and remote sync have
separate scheduling and acknowledgement.

Playback callbacks carry session generation and stable video identity. Reject
retired-owner callbacks. Ordinary played progress sends TimeChanged; seek, resume
position restoration and decoder reposition send Seek before subsequent progress.
Do not count paused time or source loading as viewing. Validate finite,
nonnegative positions/durations at the Swift/native boundary and clamp position to
known duration. Unknown duration must not trigger automatic completion.

## Watched and library policy

Use Core's strictly-greater-than-70%-watched-time rule. Seeking to the credits does
not mark a title watched. Explicit Mark watched/Mark unwatched takes precedence
over automatic updates for that video for the remainder of the current playback
session. A new explicit Play starts a fresh automatic-completion opportunity;
old callbacks cannot override a more recent manual action. Persist the manual
choice before displaying it as saved. Library membership is independent of watched
state; removing a title is not permission to delete account data or media outside
the application's cache.

## Sync and conflict policy

Guest state stays local. Connected state belongs to exactly one authenticated UID
and storage namespace; no guest import or cross-account replay is implicit. Stop
and join the old owner before changing identity. A pending write retains its owner
identity, never whichever credentials happen to be current when it executes.

Implement a durable, coalesced pending-intent journal unless executable integration
tests demonstrate equivalent guarantees in Core. Bound it to 256 title entries
and 1 MiB serialized data. Multiple operations for one title coalesce to its latest
intended state. On capacity exhaustion, retain existing entries, report that the
new change cannot be saved, and do not silently drop another title's intent.
Keep credentials and source URLs out of the journal; store it in the encrypted
account namespace, outside purgeable playback cache.

Pending local edits win over a fetched remote snapshot until acknowledged. With
no pending local edit, the fetched remote item wins. Resolve same-second conflicts
explicitly: never infer acknowledgement from timestamp equality. Use idempotent
full-state writes and retain intent until the matching revision is acknowledged;
an older successful response must not clear a newer local edit. This is an app
policy, not a claim of server-side compare-and-swap. Concurrent edits by another
client can still be superseded by a later write; document that limitation.

Retry while active using capped exponential backoff (1, 2, 4, 8, 16, then 30 seconds),
with at most one sync request in flight per owner. Persist intent, not a running
background timer. Retry on foreground/reconnection and offer an explicit retry.
Authentication rejection pauses sync until reauthentication; it does not erase
local progress. Sign-out follows the existing explicit account cleanup contract,
including removal of that account's journal after owner retirement.

## Lost close acknowledgements

An accepted final checkpoint may finish after its caller times out or cancels its
wait. Within that Core session, retain only the most recent successfully closed
playback token. A repeated Close for that token acknowledges the prior durable
save without writing again or touching a newer player. Clear this receipt on
logout; it is not persisted across sessions. Failed or still-pending writes do not
create successful receipts, and unrelated tokens still fail as stale.

This changes duplicate Close from `stale_playback` to success for that one retained
token. Repeating Start for the same active, non-closing token acknowledges the
existing player without resetting its clock. A different token cannot replace it.
Retire independently retains its most recent successful token, clears it on logout,
and acknowledges retries without touching a newer player. Retire never stands in
for a durable Close. Both retained receipts are bounded and session-local.

Core-level tests also verify that repeating exact absolute progress, pause and
seek events does not double-count watched time. Dropping a checkpoint waiter does
not roll back the admitted write: a concurrent retry remains busy until completion,
and a later checkpoint captures the newer player state. These are native-session
and real-storage proofs. The full Swift/UI timeout-and-retry journey remains a
separate qualification gate; arbitrary reordered operations are not replay-safe. The host regression holds the real sequential writer, drops the close
waiter, releases storage, then verifies retry, unchanged ciphertext and isolation
from a newer player. The Swift bridge also exercises repeated Close.

## Implementation and acceptance gates

1. Add Core-backed library actions and bounded snapshots, with real session tests
   for add/remove/watched, stale revisions and encrypted reopen.
2. Retain the chosen stream and own Core Player; bridge progress/seek/pause/end,
   proving skipped time is not watched and manual overrides survive late events.
3. Implement acknowledged local checkpoints and resume lookup. Test reordered
   writes, write failure, owner replacement and cache removal independently.
4. Add the journal and deterministic network/clock tests for offline edits,
   coalescing, capacity, stale acknowledgements, same-second conflict and UID changes.
5. Connect lifecycle and user-facing resume/library/sync-error controls. Run ten
   exit/relaunch cycles with at most 15 seconds position loss, plus pause, Home,
   interruption and abrupt termination cases. Physical suspension remains a device gate.

No account mode release or Phase 5 completion until these gates pass. Preserve
local playback if sync is disabled during development. Diagnostics expose only
error categories, phase and counters by default, never viewing IDs, URLs or tokens.
