#ifndef CUE_CORE_H
#define CUE_CORE_H
#include <stdint.h>
#include <stddef.h>
// Thread-safe; short internal locks, no waits for network/model completion.
// Handles are never reused.
uint32_t cue_core_abi_version(void);
// Starts an isolated guest session asynchronously; 0 means unavailable/capacity.
uint64_t cue_core_create(void);
// Existing app-owned absolute directory (UTF-8, no NUL), and exactly 32 key bytes.
// Inputs are copied; do not persist the key beside the ciphertext. The platform
// supplies a credential-store key. Startup/lock errors appear in the snapshot.
// Stop acknowledgement includes storage-lock release.
uint64_t cue_core_create_persistent(const uint8_t *path, size_t path_length, const uint8_t *key, size_t key_length);
// Copies input before return. JSON v1: {"version":1,"action":"set_language","language":"fra"}.
// Or {"version":1,"action":"load_addon","url":"https://host/manifest.json"}.
// 0 accepted (not completed), -1 unavailable, -2 malformed, -3 queue full.
int32_t cue_core_dispatch(uint64_t handle, const uint8_t *bytes, size_t length);
// Returns required byte count, or -1 for an invalid handle. No NUL terminator.
// Writes only when capacity is sufficient. NULL with capacity 0 queries size.
// Caller owns the buffer; pointers must refer to valid memory for the supplied size.
intptr_t cue_core_snapshot(uint64_t handle, uint8_t *output, size_t capacity);
// Stop is idempotent. Status: 0 joining, 1 fully stopped, -1 invalid handle.
int32_t cue_core_stop(uint64_t handle);
int32_t cue_core_stop_status(uint64_t handle);
// Releases only a fully stopped session: 1 released, 0 still running, -1 invalid.
int32_t cue_core_release(uint64_t handle);
#endif
