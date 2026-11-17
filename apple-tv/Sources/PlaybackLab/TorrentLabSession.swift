#if DEBUG
import Foundation
import CueTorrent

/// Main-actor UI owner; native calls schedule work on the Rust runtime.
@Observable
final class TorrentLabSession {
    private(set) var status = "Idle"
    private(set) var url: URL?
    private(set) var verifiedBytes: UInt64 = 0
    private(set) var totalBytes: UInt64 = 0
    private(set) var selectedFile: String?
    private(set) var cacheDirectory: URL?
    private(set) var pendingCleanups = 0
    private(set) var cleanupFailures = 0
    private var handle: UInt64 = 0
    private var generation = UUID()
    private var polling: Task<Void, Never>?

    func start() async {
        stop()
        let requestedGeneration = generation
        while pendingCleanups > 0 { try? await Task.sleep(for: .milliseconds(50)) }
        guard generation == requestedGeneration else { return }
        let requestedFixture = ProcessInfo.processInfo.environment["CUE_TORRENT_FIXTURE"] ?? "single"
        let fixture = ProcessInfo.processInfo.environment["CUE_LONG_TORRENT"] == "1" ? "long" : requestedFixture
        guard ["single", "long", "mkv", "hevc-mkv", "hevc-mp4"].contains(fixture) else {
            status = "Unknown torrent fixture"; return
        }
        guard let torrent = Bundle.main.url(forResource: fixture, withExtension: "torrent") else {
            status = "Missing torrent fixture"; return
        }
        let output: URL
        do { output = try await TorrentCacheStore.shared.acquire() }
        catch {
            guard generation == requestedGeneration else { return }
            status = "Media cache is unavailable; previous cleanup may be incomplete"; return
        }
        guard generation == requestedGeneration else {
            try? await TorrentCacheStore.shared.release(output)
            return
        }
        let config: [String: Any] = ["api_version": 1, "torrent_path": torrent.path, "output": output.path,
                                   "cache_root": output.deletingLastPathComponent().path,
                                   "file_index": 0, "initial_peers": ["127.0.0.1:42424"]]
        guard let data = try? JSONSerialization.data(withJSONObject: config), let json = String(data: data, encoding: .utf8) else {
            try? await TorrentCacheStore.shared.release(output); return
        }
        handle = json.withCString { cue_torrent_start($0) }
        guard handle != 0 else {
            try? await TorrentCacheStore.shared.release(output)
            status = "Engine rejected configuration"; return
        }
        cacheDirectory = output
        status = "Connecting to fixture peer"
        let current = generation
        polling = Task { [weak self] in
            while !Task.isCancelled {
                guard let self, self.generation == current else { return }
                if let pointer = cue_torrent_snapshot(self.handle) {
                    let data = Data(String(cString: pointer).utf8)
                    cue_torrent_string_free(pointer)
                    if let snapshot = try? JSONDecoder().decode(Snapshot.self, from: data) {
                        guard snapshot.api_version == 1, snapshot.session_id == self.handle else {
                            self.stop(); self.status = "Invalid engine response"; return
                        }
                        self.verifiedBytes = snapshot.verified_bytes ?? 0
                        self.totalBytes = snapshot.total_bytes ?? 0
                        self.selectedFile = snapshot.files?.first(where: { $0.index == 0 })?.path
                        self.status = snapshot.state
                        if snapshot.state == "ready", let value = snapshot.url {
                            if self.url == nil { self.url = URL(string: value) }
                        }
                        if snapshot.state == "failed" { return }
                    }
                }
                try? await Task.sleep(for: .milliseconds(100))
            }
        }
    }

    func stop() {
        generation = UUID()
        polling?.cancel(); polling = nil
        let ticket = cue_torrent_stop_ticket(handle)
        let directory = cacheDirectory
        handle = 0; cacheDirectory = nil
        if ticket != 0 {
            pendingCleanups += 1
            let cleanup = Task.detached {
                defer { cue_torrent_stop_release(ticket) }
                for _ in 0..<100 {
                    let state = cue_torrent_stop_status(ticket)
                    if state == 1 {
                        guard let directory else { return true }
                        do {
                            try await TorrentCacheStore.shared.release(directory)
                            return true
                        } catch { return false }
                    }
                    if state < 0 { return false }
                    try? await Task.sleep(for: .milliseconds(50))
                }
                // Keep files if shutdown is unconfirmed; never delete on timeout.
                return false
            }
            Task { [weak self] in
                let removed = await cleanup.value
                guard let self else { return }
                self.pendingCleanups -= 1
                if !removed { self.cleanupFailures += 1 }
            }
        }
        url = nil; status = "Idle"
        verifiedBytes = 0; totalBytes = 0; selectedFile = nil
    }
    private struct Snapshot: Decodable {
        let api_version: UInt32
        let session_id: UInt64
        let state: String
        let url: String?
        let verified_bytes: UInt64?
        let total_bytes: UInt64?
        let files: [File]?
        struct File: Decodable { let index: Int; let path: String }
    }
}
#endif
