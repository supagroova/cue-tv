import Foundation
import CueTorrent

nonisolated struct NativePlaybackSnapshot: Decodable, Sendable {
    let apiVersion: UInt32
    let sessionId: UInt64
    let state: String
    let errorCode: String?
    let url: String?
    let verifiedBytes: UInt64?
    let totalBytes: UInt64?
}

/// Owns one native playback handle. Cache release belongs to its coordinator,
/// and must happen only after stop returns successfully.
actor NativePlaybackService {
    enum Failure: Error { case stopped, malformedSnapshot, shutdownTimeout, unavailable }
    private var handle: UInt64?
    private var ticket: UInt64?
    private var retirement: Task<Void, any Error>?
    init(adopting handle: UInt64) { self.handle = handle }

    func snapshot() throws -> NativePlaybackSnapshot {
        guard let handle, retirement == nil, ticket == nil else { throw Failure.stopped }
        guard let pointer = cue_torrent_snapshot(handle) else { throw Failure.unavailable }
        defer { cue_torrent_string_free(pointer) }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let value = try? decoder.decode(NativePlaybackSnapshot.self, from: Data(String(cString: pointer).utf8)),
              value.apiVersion == 1, value.sessionId == handle else { throw Failure.malformedSnapshot }
        return value
    }

    func stop() async throws {
        guard let handle else { return }
        let task: Task<Void, any Error>
        if let retirement { task = retirement }
        else {
            let token = ticket ?? cue_torrent_stop_ticket(handle)
            guard token != 0 else { throw Failure.unavailable }
            ticket = token
            task = Task.detached { try await Self.join(token) }
            retirement = task
        }
        do {
            try await task.value
            if self.handle == handle {
                self.handle = nil; ticket = nil; retirement = nil
            }
        } catch {
            retirement = nil
            throw error
        }
    }

    private nonisolated static func join(_ ticket: UInt64) async throws {
        let deadline = ContinuousClock.now.advanced(by: .seconds(5))
        while cue_torrent_stop_status(ticket) == 0 {
            guard ContinuousClock.now < deadline else { throw Failure.shutdownTimeout }
            try await Task.sleep(for: .milliseconds(10))
        }
        guard cue_torrent_stop_status(ticket) == 1 else { throw Failure.unavailable }
        cue_torrent_stop_release(ticket)
    }

    deinit {
        if let handle, retirement == nil {
            let token = ticket ?? cue_torrent_stop_ticket(handle)
            if token != 0 {
                Task.detached {
                    defer { cue_torrent_stop_release(token) }
                    try? await Self.join(token)
                }
            }
        }
    }
}
