#ifndef CUE_TORRENT_H
#define CUE_TORRENT_H
#include <stdint.h>
// Borrowed UTF-8 JSON: api_version=1, exactly one torrent_path/magnet, output,
// file_index, initial_peers, optional discovery {trackers,dht,bootstrap}.
// Returns zero for invalid/unsupported envelopes. Work runs on the native runtime.
uint64_t cue_torrent_start(const char *config);
// Versioned snapshot includes api_version=1 and session_id matching the handle.
// Ready snapshots include files and verified/total byte counts.
// Caller owns returned string; free exactly once with cue_torrent_string_free.
char *cue_torrent_snapshot(uint64_t handle);
void cue_torrent_string_free(char *value);
// Idempotent; requests asynchronous shutdown, invalidates the handle immediately.
void cue_torrent_stop(uint64_t handle);
#endif
