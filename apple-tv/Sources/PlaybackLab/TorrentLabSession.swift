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
    private var handle: UInt64 = 0
    private var generation = UUID()
    private var polling: Task<Void, Never>?

    func start() {
        stop()
        let fixture = ProcessInfo.processInfo.environment["CUE_LONG_TORRENT"] == "1" ? "long" : "single"
        guard let torrent = Bundle.main.url(forResource: fixture, withExtension: "torrent") else {
            status = "Missing torrent fixture"; return
        }
        let output = FileManager.default.temporaryDirectory.appendingPathComponent("cue-torrent-\(UUID().uuidString)")
        let config: [String: Any] = ["api_version": 1, "torrent_path": torrent.path, "output": output.path,
                                   "file_index": 0, "initial_peers": ["127.0.0.1:42424"]]
        guard let data = try? JSONSerialization.data(withJSONObject: config), let json = String(data: data, encoding: .utf8) else { return }
        handle = json.withCString { cue_torrent_start($0) }
        guard handle != 0 else { status = "Engine rejected configuration"; return }
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
        cue_torrent_stop(handle); handle = 0
        url = nil; status = "Idle"
        verifiedBytes = 0; totalBytes = 0
    }
    private struct Snapshot: Decodable {
        let api_version: UInt32
        let session_id: UInt64
        let state: String
        let url: String?
        let verified_bytes: UInt64?
        let total_bytes: UInt64?
    }
}
#endif
