#ifndef CUE_TORRENT_H
#define CUE_TORRENT_H
#include <stdint.h>
// Borrowed null-terminated UTF-8 config. Returns zero on invalid config.
uint64_t cue_torrent_start(const char *config);
// Caller owns returned string; free exactly once with cue_torrent_string_free.
char *cue_torrent_snapshot(uint64_t handle);
void cue_torrent_string_free(char *value);
// Idempotent; requests asynchronous shutdown, invalidates the handle immediately.
void cue_torrent_stop(uint64_t handle);
#endif
