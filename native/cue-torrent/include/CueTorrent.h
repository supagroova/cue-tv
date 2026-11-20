#ifndef CUE_TORRENT_H
#define CUE_TORRENT_H
#include <stdint.h>
// Borrowed UTF-8 JSON: api_version=1, exactly one torrent_path/magnet, output,
// file_index, initial_peers, optional discovery {trackers,dht,bootstrap}.
// Optional cache_root must be the existing output directory's direct parent;
// allocated data and metadata across that root count toward the budget.
// Returns zero for invalid/unsupported envelopes. Work runs on the native runtime.
uint64_t cue_torrent_start(const char *config);
// Versioned snapshot includes api_version=1 and session_id matching the handle.
// Ready snapshots include files and verified/total byte counts.
// Failed snapshots add error_code: invalid_metadata, unsupported_format,
// source_too_large, low_storage, or read. Consumers must handle unknown codes.
// Error payloads exclude raw native error chains, source paths and URLs.
// Caller owns returned string; free exactly once with cue_torrent_string_free.
char *cue_torrent_snapshot(uint64_t handle);
void cue_torrent_string_free(char *value);
// Idempotent; requests asynchronous shutdown, invalidates the handle immediately.
void cue_torrent_stop(uint64_t handle);
// Stops and invalidates a live handle; returns a ticket, or zero if unknown/already stopped.
uint64_t cue_torrent_stop_ticket(uint64_t handle);
// 0 pending, 1 storage closed and session owner finished, -1 unknown/released.
// Acknowledges cache I/O closure, not joining every upstream network task.
int32_t cue_torrent_stop_status(uint64_t ticket);
// Required exactly once per nonzero ticket (idempotent); safe to abandon a pending ticket.
void cue_torrent_stop_release(uint64_t ticket);
#endif
