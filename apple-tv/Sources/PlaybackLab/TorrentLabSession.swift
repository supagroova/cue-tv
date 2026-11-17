#if DEBUG
import Foundation
import CueTorrent

/// Main-actor UI owner; native calls schedule work on the Rust runtime.
@Observable
final class TorrentLabSession {
    private(set) var status = "Idle"
    private(set) var url: URL?
    private var handle: UInt64 = 0
    private var generation = UUID()
    private var polling: Task<Void, Never>?

    func start() {
        stop()
        guard let torrent = Bundle.main.url(forResource: "single", withExtension: "torrent") else {
            status = "Missing torrent fixture"; return
        }
        let output = FileManager.default.temporaryDirectory.appendingPathComponent("cue-torrent-\(UUID().uuidString)")
        let config: [String: Any] = ["torrent_path": torrent.path, "output": output.path,
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
                        self.status = snapshot.state
                        if snapshot.state == "ready", let value = snapshot.url {
                            self.url = URL(string: value)
                            return
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
    }
    private struct Snapshot: Decodable { let state: String; let url: String? }
}
#endif
