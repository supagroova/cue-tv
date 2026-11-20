#ifndef CUE_CORE_H
#define CUE_CORE_H
#include <stdint.h>
#include <stddef.h>
// Thread-safe; short internal locks, no waits for network/model completion.
// Handles are never reused.
uint32_t cue_core_abi_version(void);
// Explicit app-startup opt-in: installs a process-wide Rust panic hook emitting
// only a fixed category. Replaces prior hook; does not change panic recovery.
// Call before native sessions. Idempotent; 0 on success, -1 if unavailable.
int32_t cue_core_configure_private_diagnostics(void);
// Starts an isolated guest session asynchronously; 0 means unavailable/capacity.
uint64_t cue_core_create(void);
// Existing app-owned absolute directory (UTF-8, no NUL), and exactly 32 key bytes.
// Inputs are copied; do not persist the key beside the ciphertext. The platform
// supplies a credential-store key. Startup/lock errors appear in the snapshot.
// Stop acknowledgement includes storage-lock release.
uint64_t cue_core_create_persistent(const uint8_t *path, size_t path_length, const uint8_t *key, size_t key_length);
// Same directory/key contract; requires an empty dedicated account store.
// Guest and restored sessions do not admit login. Nonempty storage fails startup.
uint64_t cue_core_create_account(const uint8_t *path, size_t path_length, const uint8_t *key, size_t key_length);
// Test-only symbol, linked only with the account-fixture Cargo feature.
// Creates an empty encrypted synthetic-account session routing API requests to
// 127.0.0.1:port, rejecting every other destination; never use real credentials.
// The feature is forbidden in physical tvOS builds. Port zero is rejected.
uint64_t cue_core_create_account_fixture(const uint8_t *path, size_t path_length, const uint8_t *key, size_t key_length, uint16_t port);
// Copies UTF-8 credentials, email 1..512 bytes, password 1..4096 bytes.
// No control characters in email, no NUL in password. Never log these inputs.
// Same admission result codes as dispatch; watch model.authenticated/model.error.
int32_t cue_core_login(uint64_t handle, const uint8_t *email, size_t email_length, const uint8_t *password, size_t password_length);
// Same bounded UTF-8 credential copy; control characters are rejected in both fields.
// For an existing paused account only; never changes account identity or namespace.
// Admission is not completion: watch model.sync.reauth_sequence/reauthenticating and errors.
// JSON v1 cancel_reauthentication with sequence targets only that attempt; never signs out.
/* Last processed cancellation sequence, or zero. Not a snapshot inference. */
uint64_t cue_core_reauthentication_cancelled(uint64_t handle);
int32_t cue_core_reauthenticate(uint64_t handle, const uint8_t *email, size_t email_length, const uint8_t *password, size_t password_length);
// Restore an existing account from the same encrypted directory/key contract.
// Expected user ID is copied UTF-8, 1..512 bytes, no control characters.
// It must come from protected app identity storage, not an arbitrary source URL.
// Identity mismatch appears as core_start_failed; this does not validate token freshness.
uint64_t cue_core_restore_account(const uint8_t *path, size_t path_length, const uint8_t *key, size_t key_length, const uint8_t *user, size_t user_length);
// Copies input before return. JSON v1: {"version":1,"action":"set_language","language":"fra"}.
// Or {"version":1,"action":"load_addon","url":"https://host/manifest.json"}.
// Install only the displayed ready preview: {"version":1,"action":"install_addon","revision":N}.
// Installed catalogue selector: {"version":1,"action":"load_catalog","index":N,"revision":N}.
// Catalogue-choice window: {"version":1,"action":"catalog_choices","offset":N,"revision":N}.
// Content window, fetching the next Core page when needed: {"version":1,"action":"catalog_items","offset":N,"revision":N}.
// Library page: {"version":1,"action":"library_window","offset":N,"revision":N}.
// Library title: {"version":1,"action":"library_details","index":N,"revision":N}.
// Title details from an index in the displayed content window: {"version":1,"action":"load_details","index":N,"revision":N}.
// Select season (null means all); resets video offset: {"version":1,"action":"details_season","season":N,"revision":N}.
// Episode/video window: {"version":1,"action":"details_videos","offset":N,"revision":N}.
// Load sources for a displayed video index (null for a movie without videos): {"version":1,"action":"load_sources","index":N,"revision":N}.
// Sign out: {"version":1,"action":"logout"}; wait for state/effects before retiring storage.
// Source window: {"version":1,"action":"source_window","offset":N,"revision":N}.
// 0 accepted (not completed), -1 unavailable, -2 malformed, -3 queue full.
int32_t cue_core_dispatch(uint64_t handle, const uint8_t *bytes, size_t length);
// Explicit private identity copy, only for protected active-account storage; never log.
// Returns required UTF-8 bytes (at most 512), 0 if no identity, -1 invalid/stopping.
// No NUL terminator. Short buffers untouched; NULL/0 queries size. No token/email returned.
intptr_t cue_core_account_identity(uint64_t handle, uint8_t *output, size_t capacity);
// Returns required byte count, or -1 for an invalid handle. No NUL terminator.
// Writes only when capacity is sufficient. NULL with capacity 0 queries size.
// Caller owns the buffer; pointers must refer to valid memory for the supplied size.
intptr_t cue_core_snapshot(uint64_t handle, uint8_t *output, size_t capacity);
// One private handoff slot per session; 0 is busy/unavailable. Tickets are never reused.
uint64_t cue_core_request_torrent(uint64_t handle, size_t index, uint64_t revision);
// HTTPS request uses the same exclusive slot, ticket validation and take/discard functions.
uint64_t cue_core_request_https(uint64_t handle, size_t index, uint64_t revision);
/* Bounded version-1 JSON identity {meta_id,video_id,kind}; shares take/discard.
 * Discard cancels active discovery. Input is copied; zero is invalid/busy. */
uint64_t cue_core_request_subtitles(uint64_t handle, const uint8_t *input, size_t length);
/* Selected-title membership save: member must be 0 or 1. Uses the exclusive
 * take/discard slot. Success request is {member:bool} after effects settle.
 * Cancellation/timeout does not roll back an accepted mutation; refresh state.
 * Zero means invalid/busy/unavailable. */
uint64_t cue_core_request_library(uint64_t handle, uint8_t member, uint64_t revision);
/* Watched save shares take/discard. watched is 0 or 1; length 0 denotes film.
 * Otherwise video points to length readable UTF-8 bytes (1..512), copied on call.
 * Success request is {watched:bool,video_id:string|null} after effects settle.
 * Cancellation/timeout does not roll back an accepted mutation.
 * Zero means invalid/busy/unavailable. */
uint64_t cue_core_request_watched(uint64_t handle, const uint8_t *video, size_t length,
                                  uint8_t watched, uint64_t revision);
/* Playback receipt: action 0=start,1=seek,2=played progress,3=pause,4=unpause,
 * 5=checkpoint,6=durable close,7=retire without saving (abort only).
 * token is the nonzero playback_token from a prepared source in this session.
 * time/duration are milliseconds <= INT64_MAX, zero unless action is 1 or 2.
 * Same exclusive take/discard slot. Success request is {token,action,progress};
 * progress is {time,duration,time_watched,watched} for actions 1..4, else null.
 * Checkpoint/close success follows durable local storage. Cancel/timeout does not
 * roll back an admitted action. Close freezes progress and retires only on success.
 * Zero means invalid/busy/unavailable. */
uint64_t cue_core_request_playback(uint64_t handle, uint64_t token, uint8_t action,
                                 uint64_t time, uint64_t duration);
// JSON v1 reply includes session, ticket, request or fixed error; at most 256 KiB.
// Returns 0 pending, -1 invalid, or byte count. Only a full non-null copy consumes it.
// NULL/0 queries length. Undersized buffers stay untouched. Stop discards the reply.
intptr_t cue_core_take_torrent(uint64_t handle, uint64_t ticket, uint8_t *output, size_t capacity);
int32_t cue_core_discard_torrent(uint64_t handle, uint64_t ticket);
// Stop is idempotent. Status: 0 joining, 1 fully stopped, -1 invalid handle.
int32_t cue_core_stop(uint64_t handle);
int32_t cue_core_stop_status(uint64_t handle);
// Releases only a fully stopped session: 1 released, 0 still running, -1 invalid.
int32_t cue_core_release(uint64_t handle);
#endif
